mod file;
mod home;
mod not_found;
mod post;
mod static_files;
mod upload;

pub use file::*;
pub use home::*;
pub use not_found::*;
pub use post::*;
pub use static_files::*;
pub use upload::*;

use crate::domain::AppState;
use crate::http::{HttpError, HttpRequest, Response};

pub fn route(request: &HttpRequest, state: &mut AppState) -> Result<Response, HttpError> {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => home_page_handler(request, state),
        ("POST", "/") => upload_post_handler(request, state),
        ("GET", path) if path.starts_with("/static/") => static_handler(request),
        ("GET", path) if path.starts_with("/post/") => {
            if let Some(slug) = path.strip_prefix("/post/") {
                if !slug.is_empty() {
                    return post_page_handler(request, slug, state);
                }
            }
            not_found_handler(request, state)
        }
        ("GET", path) if path.starts_with("/file/") => {
            if let Some(slug) = path.strip_prefix("/file/") {
                if !slug.is_empty() {
                    return file_handler(request, state);
                }
            }
            not_found_handler(request, state)
        }
        _ => not_found_handler(request, state),
    }
}
