//struct Worker {
    //id: u32,
    //thread: Option<thread::JoinHandle<()>>,
//}
use std::io;
use std::str::Utf8Error;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::str;
use std::rc::Rc;
use std::sync::Mutex;

enum Route {
    StaticFile(String),
    Home,
    UploadPage,
    UploadPost,
    NotFound,
}

#[derive(Debug, thiserror::Error)]
enum HttpError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] Utf8Error),

    #[error("Invalid HTTP request: {0}")]
    InvalidRequest(String), 

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
}
impl HttpError {
    pub fn status_code(&self) -> u16 {
        match self {
            HttpError::BadRequest(_) => 400,
            HttpError::NotFound(_) => 404,
            HttpError::Io(_) => 500,
            HttpError::Utf8(_) => 400,
            HttpError::InvalidRequest(_) => 400,
            HttpError::EmptyRequest => 400,
            HttpError::InvalidRequestLine => 400,
            HttpError::InternalServerError(_) => 500,
        }
    }
}
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
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("server listening on port 8080");
    let app_state = Rc::new(Mutex::new(AppState { 
        posts: Vec::new()
    }));
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => 
            handle_connection(stream, app_state.clone()),
            Err(e) => 
                eprintln!("error: {:?}", e)
            }
        }
    Ok(())
}

fn router(method: &str, path: &str) -> Route {
    match (method, path) {
        ("GET", "/") => Route::Home,
        ("GET", "/upload") => Route::UploadPage,
        ("POST", "/upload") => Route::UploadPost,
        ("GET", path) if path.starts_with("/static/") => {
            match path.strip_prefix("/static/") {
                Some(filename) if !filename.is_empty() && !filename.contains("..") => 
                Route::StaticFile(filename.to_string()),
                _ => Route::NotFound,
            }
        }
        _ => Route::NotFound,
    }

}

fn parse_request_line(request_str: &str) -> Result<(String, String), HttpError> {
    let first_line = request_str.lines().next().ok_or(HttpError::EmptyRequest)?;
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(HttpError::InvalidRequestLine)
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn read_request(stream: &mut TcpStream) -> Result<String, HttpError> {
    let mut buffer = [0; 65536];
    let bytes_read = stream.read(&mut buffer)?;
    let request_str = str::from_utf8(&buffer[..bytes_read])?;
    Ok(request_str.to_string())
}

fn read_raw_request(stream: &mut TcpStream) -> Result<Vec<u8>, HttpError> {
    let mut buffer = [0u8; 65536];
    let bytes_read = stream.read(&mut buffer)?;
    Ok(buffer[..bytes_read].to_vec())
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

fn send_binary_response(stream: &mut TcpStream, status_line: &str, content_type: &str, data: &[u8]) {
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

fn handle_static(filename: &str) -> Result<Vec<u8>, HttpError> {
    use std::path::Path;
    use std::fs;
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
                Err(HttpError::NotFound(format!("File '{}' not found", filename)))
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

fn handle_connection(mut stream: TcpStream, state: Rc<Mutex<AppState>>) -> () {
    let request_str = match read_request(&mut stream) {
        Ok(request_str) => request_str,
        Err(e) => {
            let status_line = format!("HTTP/1.1 {} Bad Request", e.status_code());
            send_response(&mut stream, &status_line, &e.to_string());
        return;
    }
};
    let (method, path) = match parse_request_line(&request_str) {
        Ok((m, p)) => (m, p),
        Err(e) => {
            let status_line = format!("HTTP/1.1 {} Bad Request", e.status_code());
            send_response(&mut stream, &status_line, &e.to_string());
        return;
    }
};
let current_posts = {
    let guard = state.lock().unwrap();
    guard.posts.len()
};
let route = router(&method, &path);
let (status_line, content) = match route {
        Route::Home => {
            let html = format!(r##"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <title>0xrugger</title>
    <script src="/static/main.js"></script>
</head>
<body>
    <h1>X RUGGER MUSIC</h1>
    <p>I'm just doin a little trap :33</p>
    <button onclick="showUploadForm()">Upload</button>
<div id="uploadForm" style="display:none;">
  <textarea id="postText" name="text"
  placeholder="type your text here" rows="5" cols="50" maxlength="1000" oninput="updateCounter()"></textarea>
  <div id="counter">0/1000</div>
  <input type="file" id="fileInput" multiple>
  <button onclick="uploadFile()">Publish</button>
</div>
 <div id="feed" class="posts-container">
    <h2>Recent Posts</h2>
    <p>Total posts: {}</p>
  </div>
</body>
</html>"##, current_posts);
        ("HTTP/1.1 200 OK".to_string(), html)
        },
        Route::UploadPage => {
            let html = r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <title>Loading...</title>
</head>
<body>
    <h1>Upload</h1>
    <p>soon</p>
    <a href="/">to the main page</a>
</body>
</html>"#;
            ("HTTP/1.1 200 OK".to_string(), html.to_string())
        },
        Route::UploadPost => {
            let html = r#"<!DOCTYPE html>
    <html>
    <body>
        <h1>Uploaded successfully</h1>
        <a href="/">to the main page</a>
    </body>
    </html>"#;
    
    ("HTTP/1.1 200 OK".to_string(), html.to_string())
},
       Route::StaticFile(filename) => {
        match handle_static(&filename) {
        Ok(file_data) => {
            let content_type = get_content_type(&filename);
            send_binary_response(
                &mut stream,
                "HTTP/1.1 200 OK",
                content_type,
                &file_data
            );
            return;
        }
        Err(e) => {
            let status_line = format!("HTTP/1.1 {}", e.status_code());
            let error_html = format!("Error: {}", e);
            (status_line, error_html)
        }
    }
},
        Route::NotFound => {
            let html = r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <title>not found</title>
</head>
<body>
    <h1>404 - page not found</h1>
    <p>this path is not exist</p>
    <a href="/">to the main page</a>
</body>
</html>"#;
            ("HTTP/1.1 404 NOT FOUND".to_string(), html.to_string())
        },
    };
    send_response(&mut stream, &status_line, &content);
}
