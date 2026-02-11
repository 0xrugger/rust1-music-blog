use std::fs;
use std::path::Path;

use crate::http::{HttpError, HttpRequest, Response};

pub fn static_handler(req: &HttpRequest) -> Result<Response, HttpError> {
    let filename = req
        .path
        .strip_prefix("/static/")
        .ok_or_else(|| HttpError::NotFound("Invalid static path".to_string()))?;

    let data = handle_static(filename)?;
    let content_type = get_content_type(filename);
    Ok(Response::binary(200, data, content_type))
}

fn handle_static(filename: &str) -> Result<Vec<u8>, HttpError> {
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
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(HttpError::NotFound(format!(
            "File '{}' not found",
            filename
        ))),
        Err(e) => Err(HttpError::Io(e)),
    }
}

pub fn get_content_type(filename: &str) -> &'static str {
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
