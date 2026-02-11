use crate::HttpError;
use crate::http::error::Error;
use crate::multi_exp;
use percent_encoding::percent_decode_str;
use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;
use std::str;

#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub query_params: HashMap<String, String>,
    pub content_type: Option<String>,
}

impl HttpRequest {
    pub fn from_tcp_stream(stream: &mut TcpStream) -> Result<Self, Error> {
        let mut buffer = Vec::new();
        let mut temp_buf = [0u8; 8192];
        let mut headers_end_pos = 0;
        loop {
            let bytes_read = stream.read(&mut temp_buf)?;
            if bytes_read == 0 {
                break;
            }
            buffer.extend_from_slice(&temp_buf[..bytes_read]);

            if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                headers_end_pos = pos + 4;
                break;
            }
        }

        if headers_end_pos == 0 {
            return Err(Error::BadRequest("No header delimiter found".to_string()));
        }

        let headers_data = &buffer[..headers_end_pos];
        let headers_str = str::from_utf8(headers_data)?;
        let mut lines = headers_str.lines();
        let first_line = lines.next().ok_or(Error::EmptyRequest)?;
        let mut parts = first_line.split_whitespace();

        let method = parts.next().ok_or(Error::InvalidRequestLine)?.to_string();
        let path_with_query = parts.next().ok_or(Error::InvalidRequestLine)?.to_string();

        let (path, query) = if let Some(pos) = path_with_query.find('?') {
            let (p, q) = path_with_query.split_at(pos);
            (p.to_string(), q[1..].to_string())
        } else {
            (path_with_query, String::new())
        };

        let query_params = Self::parse_query_string(&query);
        let mut content_length: usize = 0;
        let mut headers = Vec::new();
        let mut content_type = None;

        for line in lines {
            if line.trim().is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                let name = name.trim().to_string();
                let value = value.trim().to_string();
                headers.push((name.clone(), value.clone()));

                if name.eq_ignore_ascii_case("content-length") {
                    content_length = value.parse().unwrap_or(0);
                }
                if name.eq_ignore_ascii_case("content-type") {
                    content_type = Some(value.clone());
                }
            }
        }

        let current_body_len = buffer.len().saturating_sub(headers_end_pos);
        let mut remaining = content_length.saturating_sub(current_body_len);

        while remaining > 0 {
            let bytes_read = stream.read(&mut temp_buf)?;
            if bytes_read == 0 {
                break;
            }
            buffer.extend_from_slice(&temp_buf[..bytes_read]);
            remaining = remaining.saturating_sub(bytes_read);
        }

        let body = buffer[headers_end_pos..].to_vec();

        Ok(HttpRequest {
            method,
            path,
            headers,
            body,
            query_params,
            content_type,
        })
    }

    pub fn get_query_param(&self, param: &str) -> Option<&str> {
        self.query_params.get(param).map(|s| s.as_str())
    }

    fn parse_query_string(query: &str) -> HashMap<String, String> {
        let mut result = HashMap::new();
        if query.is_empty() {
            return result;
        }

        for part in query.split('&') {
            if part.is_empty() {
                continue;
            }
            let (key_raw, value_raw) = match part.find('=') {
                Some(pos) => {
                    let (k, v) = part.split_at(pos);
                    (k, &v[1..])
                }
                None => (part, ""),
            };
            let decoded_key = percent_decode_str(key_raw).decode_utf8_lossy();
            let decoded_value = percent_decode_str(value_raw).decode_utf8_lossy();
            result.insert(decoded_key.to_string(), decoded_value.to_string());
        }
        result
    }

    pub fn parse_urlencoded(body: &[u8]) -> Result<HashMap<String, String>, Error> {
        let body_str = str::from_utf8(body)?.replace('+', " ");
        let mut map = HashMap::new();
        for pair in body_str.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                let decoded_key = percent_decode_str(key).decode_utf8_lossy();
                let decoded_value = percent_decode_str(value).decode_utf8_lossy();
                map.insert(decoded_key.to_string(), decoded_value.to_string());
            }
        }
        Ok(map)
    }
    pub fn parse_multipart(&self) -> Result<HashMap<String, multi_exp::FormField>, HttpError> {
        let content_type = self
            .content_type
            .as_ref()
            .ok_or_else(|| HttpError::BadRequest("No Content-Type header".to_string()))?;

        if !content_type.starts_with("multipart/form-data") {
            return Err(HttpError::BadRequest(format!(
                "Expected multipart/form-data, got {}",
                content_type
            )));
        }

        let boundary = multi_exp::extract_boundary(content_type)
            .map_err(|e| HttpError::MultipartError(format!("Boundary extraction: {}", e)))?;

        match multi_exp::parse_multipart(&self.body, &boundary) {
            Ok(fields) => Ok(fields),
            Err(e) => {
                if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                    if io_err.kind() == std::io::ErrorKind::UnexpectedEof {
                        return Err(HttpError::BadRequest(
                            "Incomplete multipart request: client closed connection prematurely"
                                .to_string(),
                        ));
                    }
                }
                Err(HttpError::MultipartError(format!("Parsing: {}", e)))
            }
        }
    }
}
