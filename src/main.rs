//struct Worker {
//id: u32,
//thread: Option<thread::JoinHandle<()>>,
//}
use askama::Template;
use std::io;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::str;
use std::str::Utf8Error;
use std::sync::Mutex;

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpRequest {
    pub fn from_tcp_stream(stream: &mut TcpStream) -> Result<HttpRequest, HttpError> {
        let mut buffer = [0u8; 65536];
        let bytes_read = stream.read(&mut buffer)?;
        let data = &buffer[..bytes_read];
        let headers_end = data
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|pos| pos + 4)
            .ok_or(HttpError::BadRequest(
                "No header delimiter found".to_string(),
            ))?;
        let headers_data = &data[..headers_end];
        let headers_str = std::str::from_utf8(headers_data)?;
        let first_line = headers_str.lines().next().ok_or(HttpError::EmptyRequest)?;

        let mut parts = first_line.split_whitespace();
        let method = parts
            .next()
            .ok_or(HttpError::InvalidRequestLine)?
            .to_string();
        let path = parts
            .next()
            .ok_or(HttpError::InvalidRequestLine)?
            .to_string();
        let mut content_length = 0;
        let mut headers = Vec::new();

        for line in headers_str.lines().skip(1) {
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
            }
        }
        let body_start = headers_end;
        let body_end = std::cmp::min(data.len(), body_start + content_length);
        let body = data[body_start..body_end].to_vec();

        Ok(HttpRequest {
            method,
            path,
            headers,
            body,
        })
    }
}
struct Response {
    status: u16,
    body: ResponseBody,
}
enum ResponseBody {
    Html(String),
    Binary(Vec<u8>, &'static str),
}

impl Response {
    fn html(status: u16, content: String) -> Self {
        Response {
            status,
            body: ResponseBody::Html(content),
        }
    }

    fn not_found_with_html(html: String) -> Self {
        Self::html(404, html)
    }

    fn binary(status: u16, data: Vec<u8>, content_type: &'static str) -> Self {
        Response {
            status,
            body: ResponseBody::Binary(data, content_type),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum HttpError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] Utf8Error),

    #[error("Empty request")]
    EmptyRequest,

    #[error("Invalid request line")]
    InvalidRequestLine,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Template rendering error: {0}")]
    TemplateError(String),
}
impl HttpError {
    pub fn status_code(&self) -> u16 {
        match self {
            HttpError::BadRequest(_) => 400,
            HttpError::NotFound(_) => 404,
            HttpError::Io(_) => 500,
            HttpError::Utf8(_) => 400,
            HttpError::EmptyRequest => 400,
            HttpError::InvalidRequestLine => 400,
            HttpError::InternalServerError(_) => 500,
            HttpError::TemplateError(_) => 500,
        }
    }
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    posts_count: usize,
}
#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate;
#[derive(Template)]
#[template(path = "upload_success.html")]
struct UploadSuccess;
#[derive(Template)]
#[template(path = "upload.html")]
struct UploadTemplate;

struct Post {
    id: u32,
    text: String,
    files: Vec<FileData>,
}
struct FileData {
    filename: String,
    content_type: String,
    data: Vec<u8>,
}
struct AppState {
    posts: Vec<Post>,
}

fn main() -> std::io::Result<()> {
    let app_state = Rc::new(Mutex::new(AppState { posts: Vec::new() }));
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("server listening on port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = handle_connection(stream, app_state.clone()) {
                    eprintln!("Connection handler error: {}", e);
                }
            }
            Err(e) => {
                eprintln!("TCP accept error: {:?}", e);
            }
        }
    }
    Ok(())
}

fn send_response(stream: &mut TcpStream, status_line: &str, content: &str) {
    let response = format!(
        "{}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        content.len(),
        content
    );
    if let Err(e) = stream.write_all(response.as_bytes()) {
        eprintln!("Failed to send response: {}", e);
    }
}

fn send_binary_response(
    stream: &mut TcpStream,
    status_line: &str,
    content_type: &str,
    data: &[u8],
) {
    let header = format!(
        "{}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
        status_line,
        content_type,
        data.len()
    );
    if let Err(e) = stream.write_all(header.as_bytes()) {
        eprintln!("Failed to send header: {}", e);
        return;
    }
    if let Err(e) = stream.write_all(data) {
        eprintln!("Failed to send data: {}", e);
    }
}

fn home_page_handler(req: &HttpRequest, state: &AppState) -> Result<Response, HttpError> {
    let template = HomeTemplate {
        posts_count: state.posts.len(),
    };
    let html = template
        .render()
        .map_err(|e| HttpError::InternalServerError(format!("Template error: {}", e)))?;
    Ok(Response::html(200, html))
}

fn upload_page_handler(req: &HttpRequest) -> Result<Response, HttpError> {
    let template = UploadTemplate;
    let html = template
        .render()
        .map_err(|e| HttpError::TemplateError(e.to_string()))?;
    Ok(Response::html(200, html.to_string()))
}

fn upload_post_handler(req: &HttpRequest, state: &mut AppState) -> Result<Response, HttpError> {
    let template = UploadSuccess;
    let html = template
        .render()
        .map_err(|e| HttpError::InternalServerError(format!("Template error: {}", e)))?;
    Ok(Response::html(200, html.to_string()))
}

fn not_found_handler(req: &HttpRequest) -> Result<Response, HttpError> {
    let template = NotFoundTemplate;
    let html = template
        .render()
        .map_err(|e| HttpError::InternalServerError(format!("Template error: {}", e)))?;
    Ok(Response::not_found_with_html(html.to_string()))
}

fn static_handler(req: &HttpRequest) -> Result<Response, HttpError> {
    let filename = req
        .path
        .strip_prefix("/static/")
        .ok_or_else(|| HttpError::NotFound("Invalid static path".to_string()))?;

    let data = handle_static(filename)?;
    let content_type = get_content_type(filename);
    Ok(Response::binary(200, data, content_type))
}

fn handle_static(filename: &str) -> Result<Vec<u8>, HttpError> {
    use std::fs;
    use std::path::Path;
    if filename.contains("..") {
        return Err(HttpError::NotFound("Invalid path".to_string()));
    }
    let base_path = Path::new("./static");
    let full_path = base_path.join(filename);
    if !full_path.starts_with(base_path) {
        return Err(HttpError::NotFound("Invalid path".to_string()));
    }
    match fs::read(&full_path) {
        Ok(data) => Ok(data),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(HttpError::NotFound(format!(
                    "File '{}' not found",
                    filename
                )))
            } else {
                Err(HttpError::Io(e))
            }
        }
    }
}

fn get_content_type(filename: &str) -> &'static str {
    use std::path::Path;
    let path = Path::new(filename);
    match path.extension().and_then(|s| s.to_str()) {
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("html") => "text/html",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
}

fn handle_connection(mut stream: TcpStream, state: Rc<Mutex<AppState>>) -> Result<(), HttpError> {
    let request = match HttpRequest::from_tcp_stream(&mut stream) {
        Ok(req) => req,
        Err(e) => {
            let status_line = format!("HTTP/1.1 {} Bad Request", e.status_code());
            send_response(&mut stream, &status_line, &e.to_string());
            return Ok(());
        }
    };

    let result: Result<Response, HttpError> = match (request.method.as_str(), request.path.as_str())
    {
        ("GET", "/") => {
            let state_guard = state
                .lock()
                .map_err(|e| HttpError::InternalServerError(format!("Mutex poison: {}", e)))?;
            home_page_handler(&request, &state_guard)
        }
        ("GET", "/upload") => upload_page_handler(&request),
        ("POST", "/upload") => {
            let mut state_guard = state
                .lock()
                .map_err(|e| HttpError::InternalServerError(format!("Mutex poison: {}", e)))?;
            upload_post_handler(&request, &mut state_guard)
        }
        ("GET", path) if path.starts_with("/static/") => static_handler(&request),
        _ => not_found_handler(&request),
    };

    match result {
        Ok(response) => {
            let status_line = format!("HTTP/1.1 {}", response.status);
            match response.body {
                ResponseBody::Html(html) => {
                    send_response(&mut stream, &status_line, &html);
                }
                ResponseBody::Binary(data, content_type) => {
                    send_binary_response(&mut stream, &status_line, content_type, &data);
                }
            }
        }
        Err(e) => {
            let status_line = format!("HTTP/1.1 {}", e.status_code());
            send_response(&mut stream, &status_line, &e.to_string());
        }
    }
    Ok(())
}
