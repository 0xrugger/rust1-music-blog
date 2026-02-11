//
mod http;
mod multi_exp;
use crate::multi_exp::FormField;

use askama::Template;
use http::{Error as HttpError, HttpRequest, Response, ResponseBody, send_binary, send_html};
use std::collections::HashMap;
use std::io;
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::Mutex;

fn slugify(text: &str) -> String {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().take(3).collect();
    let with_dashes = words.join("-");
    let filtered: String = with_dashes
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    let mut result = filtered;
    while result.contains("--") {
        result = result.replace("--", "-");
    }
    result = result.trim_matches('-').to_string();
    if result.is_empty() {
        return "post".to_string();
    }
    if result.len() > 50 {
        result.truncate(50);

        result = result.trim_end_matches('-').to_string();
    }
    result
}
fn validate_slug(slug: &str) -> Result<(), HttpError> {
    if slug.is_empty() {
        return Err(HttpError::ValidationError("Slug empty".to_string()));
    };
    if slug.len() < 3 || slug.len() > 50 {
        return Err(HttpError::ValidationError("Slug lenght error".to_string()));
    };
    if slug.starts_with('-') || slug.ends_with('-') {
        return Err(HttpError::ValidationError(
            "Slug start/end contains '-'".to_string(),
        ));
    };
    if slug.contains("--") {
        return Err(HttpError::ValidationError("Slug contains '--'".to_string()));
    };
    for ch in slug.chars() {
        if !(ch.is_ascii_alphanumeric() || ch == '-') {
            return Err(HttpError::ValidationError(
                "Slug grammar is not correct".to_string(),
            ));
        };
    }
    Ok(())
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    posts_count: usize,
    nav_items: Vec<NavItem>,
    current_page: String,
    show_upload_success: bool,
}
#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate {
    nav_items: Vec<NavItem>,
    current_page: String,
}
#[derive(Template)]
#[template(path = "post.html")]
struct PostTemplate<'a> {
    post: &'a Post,
    nav_items: Vec<NavItem>,
    current_page: String,
}
struct Post {
    id: u32,
    slug: String,
    text: String,
    title: String,
    filename: Option<String>,
    file_data: Option<Vec<u8>>,
}
struct NavItem {
    title: String,
    url: String,
    is_current: bool,
}
struct AppState {
    posts: Vec<Post>,
}

impl AppState {
    pub fn find_post_by_slug(&self, slug: &str) -> Option<&Post> {
        self.posts.iter().find(|post| post.slug == slug)
    }
    pub fn generate_navigation(&self, current_path: &str) -> Vec<NavItem> {
        let mut result = Vec::new();
        for (index, post) in self.posts.iter().enumerate() {
            let title = format!("POST {}", index + 1);
            let url = format!("/post/{}", post.slug);
            let is_current = current_path == url;
            result.push(NavItem {
                title,
                url,
                is_current,
            });
        }
        result
    }
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

fn home_page_handler(req: &HttpRequest, state: &AppState) -> Result<Response, HttpError> {
    let show_upload_success = req.get_query_param("upload_success") == Some("true");
    let nav_items = state.generate_navigation("/");
    let template = HomeTemplate {
        posts_count: state.posts.len(),
        nav_items,
        current_page: "/".to_string(),
        show_upload_success,
    };
    let html = template.render()?;
    Ok(Response::html(200, html))
}

fn upload_urlencoded_handler(
    req: &HttpRequest,
    state: &mut AppState,
) -> Result<Response, HttpError> {
    let form_data = HttpRequest::parse_urlencoded(&req.body)
        .map_err(|e| HttpError::BadRequest("URL Encoding error".to_string()))?;
    let text = form_data
        .get("text")
        .ok_or(HttpError::BadRequest("Text undecoded".to_string()))?
        .clone();
    let title = form_data
        .get("title")
        .ok_or(HttpError::BadRequest("Title undecoded".to_string()))?
        .clone();
    let slug = slugify(&title);
    validate_slug(&slug)?;
    let id = state.posts.len() as u32 + 1;
    let post = Post {
        id: id,
        slug: slug,
        text: text,
        title: title,
        filename: None,
        file_data: None,
    };
    state.posts.push(post);
    Ok(Response::redirect("/?upload_success=true"))
}

fn upload_multipart_handler(
    req: &HttpRequest,
    state: &mut AppState,
) -> Result<Response, HttpError> {
    let fields = req.parse_multipart()?;
    let title = extract_text_field(&fields, "title")?;
    let text = extract_text_field(&fields, "text")?;
    let (filename, file_data) = match fields.get("image") {
        Some(FormField::File { filename, data, .. }) => {
            validate_image(data, filename)?;
            (Some(filename.clone()), Some(data.clone()))
        }
        _ => (None, None),
    };

    let slug = slugify(&title);
    validate_slug(&slug)?;
    let id = state.posts.len() as u32 + 1;

    let post = Post {
        id,
        slug,
        text,
        title,
        filename,
        file_data,
    };

    state.posts.push(post);
    Ok(Response::redirect("/?upload_success=true"))
}

fn validate_image(data: &[u8], filename: &str) -> Result<(), HttpError> {
    const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
    if data.len() > MAX_FILE_SIZE {
        return Err(HttpError::BadRequest(format!(
            "Max file size: {} mb",
            MAX_FILE_SIZE / (1024 * 1024)
        )));
    }
    let allowed_extensions = ["jpg", "jpeg", "png"];
    let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    if !allowed_extensions.contains(&extension.as_str()) {
        return Err(HttpError::BadRequest(format!(
            "Incorrect file extension: {}. U can try: {}",
            extension,
            allowed_extensions.join(", ")
        )));
    }
    if data.len() < 8 {
        return Err(HttpError::BadRequest(
            "file too short to validate".to_string(),
        ));
    }

    match extension.as_str() {
        "png" => {
            let png_signature: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
            if data[0..8] != png_signature {
                return Err(HttpError::BadRequest(
                    "Incorrect signature of file.png".to_string(),
                ));
            }
        }
        "jpg" | "jpeg" => {
            if data.len() < 2 {
                return Err(HttpError::BadRequest(
                    "File to short to validate signature JPEG".to_string(),
                ));
            }
            let jpeg_signature: [u8; 2] = [255, 216];
            if data[0..2] != jpeg_signature {
                return Err(HttpError::BadRequest(
                    "Incorrect signature of file.jpeg".to_string(),
                ));
            }
        }
        _ => {
            return Err(HttpError::BadRequest("Unsupported extension".to_string()));
        }
    }

    Ok(())
}
fn extract_text_field(
    fields: &HashMap<String, multi_exp::FormField>,
    field_name: &str,
) -> Result<String, HttpError> {
    match fields.get(field_name) {
        Some(FormField::Text(text)) => Ok(text.clone()),
        Some(FormField::File { .. }) => Err(HttpError::BadRequest(format!(
            "Field '{}' should be text, not file",
            field_name
        ))),
        None => Err(HttpError::BadRequest(format!(
            "Missing required field: {}",
            field_name
        ))),
    }
}

fn upload_post_handler(req: &HttpRequest, state: &mut AppState) -> Result<Response, HttpError> {
    match req.content_type.as_deref() {
        Some(content_type) if content_type.starts_with("multipart/form-data") => {
            upload_multipart_handler(req, state)
        }
        Some(content_type) if content_type == "application/x-www-form-urlencoded" => {
            upload_urlencoded_handler(req, state)
        }
        Some(content_type) => Err(HttpError::BadRequest(format!(
            "Unsupported content type: {}",
            content_type
        ))),
        None => Err(HttpError::BadRequest(
            "Missing Content-Type header".to_string(),
        )),
    }
}

fn post_page_handler(
    request: &HttpRequest,
    slug: &str,
    state: &AppState,
) -> Result<Response, HttpError> {
    let post = state.find_post_by_slug(slug);
    if post.is_none() {
        return not_found_handler(request, state);
    }
    let nav_items = state.generate_navigation(&format!("/post/{}", slug));
    let template = PostTemplate {
        post: post.unwrap(),
        nav_items,
        current_page: format!("/post/{}", slug),
    };
    let html = template.render()?;

    Ok(Response::html(200, html))
}

fn not_found_handler(req: &HttpRequest, state: &AppState) -> Result<Response, HttpError> {
    let nav_items = state.generate_navigation(&req.path);
    let template = NotFoundTemplate {
        nav_items,
        current_page: req.path.clone(),
    };
    let html = template.render()?;
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
fn file_handler(req: &HttpRequest, state: &AppState) -> Result<Response, HttpError> {
    let slug = req
        .path
        .strip_prefix("/file/")
        .ok_or_else(|| HttpError::NotFound("Invalid file path".to_string()))?;

    let post = state
        .find_post_by_slug(slug)
        .ok_or_else(|| HttpError::NotFound(format!("Post {} not found", slug)))?;

    match (&post.filename, &post.file_data) {
        (Some(filename), Some(data)) => {
            let content_type = get_content_type(filename);
            Ok(Response::binary(200, data.clone(), content_type))
        }
        _ => Err(HttpError::NotFound("No file attached".to_string())),
    }
}

fn handle_connection(mut stream: TcpStream, state: Rc<Mutex<AppState>>) -> Result<(), HttpError> {
    let request = match HttpRequest::from_tcp_stream(&mut stream) {
        Ok(req) => req,
        Err(e) => {
            let status_line = format!("HTTP/1.1 {} Bad Request", e.status_code());
            send_html(&mut stream, &status_line, &e.to_string());
            return Ok(());
        }
    };

    let mut state_guard = state
        .lock()
        .map_err(|e| HttpError::InternalServerError(format!("Mutex poison: {}", e)))?;
    let state_ref = &*state_guard;
    let state_mut = &mut *state_guard;
    let result = match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => home_page_handler(&request, &state_guard),
        ("POST", "/") => upload_post_handler(&request, &mut state_guard),
        ("GET", path) if path.starts_with("/static/") => static_handler(&request),
        ("GET", path) if path.starts_with("/post/") => match path.strip_prefix("/post/") {
            Some(slug) if !slug.is_empty() => post_page_handler(&request, slug, &state_guard),
            _ => not_found_handler(&request, &state_guard),
        },
        ("GET", path) if path.starts_with("/file/") => file_handler(&request, &state_guard),
        _ => not_found_handler(&request, &state_guard),
    };

    match result {
        Ok(response) => {
            let status_line = format!("HTTP/1.1 {}", response.status);
            match response.body {
                ResponseBody::Html(html) => {
                    send_html(&mut stream, &status_line, &html);
                }
                ResponseBody::Binary(data, content_type) => {
                    send_binary(&mut stream, &status_line, content_type, &data);
                }
            }
        }
        Err(e) => {
            let status_line = format!("HTTP/1.1 {}", e.status_code());
            send_html(&mut stream, &status_line, &e.to_string());
        }
    }
    Ok(())
}
