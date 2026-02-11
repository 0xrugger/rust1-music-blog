use std::io::Write;
use std::net::TcpStream;

pub enum ResponseBody {
    Html(String),
    Binary(Vec<u8>, &'static str),
}

pub struct Response {
    pub status: u16,
    pub body: ResponseBody,
}

impl Response {
    pub fn html(status: u16, content: String) -> Self {
        Response {
            status,
            body: ResponseBody::Html(content),
        }
    }

    pub fn not_found_with_html(html: String) -> Self {
        Self::html(404, html)
    }

    pub fn binary(status: u16, data: Vec<u8>, content_type: &'static str) -> Self {
        Response {
            status,
            body: ResponseBody::Binary(data, content_type),
        }
    }

    pub fn redirect(location: &str) -> Self {
        let html = format!(
            "<html><head>\
             <meta http-equiv=\"refresh\" content=\"0; url={}\">\
             </head><body>\
             Redirecting to <a href=\"{}\">{}</a>\
             </body></html>",
            location, location, location
        );
        Response::html(303, html)
    }
}

pub fn send_html(stream: &mut TcpStream, status_line: &str, content: &str) {
    let response = format!(
        "{}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        content.len(),
        content
    );
    let _ = stream.write_all(response.as_bytes());
}

pub fn send_binary(stream: &mut TcpStream, status_line: &str, content_type: &str, data: &[u8]) {
    let header = format!(
        "{}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
        status_line,
        content_type,
        data.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(data);
}
