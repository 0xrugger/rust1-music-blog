use crate::domain::AppState;
use crate::http::{HttpError, HttpRequest, Response};
use crate::route::static_files::get_content_type;

pub fn file_handler(req: &HttpRequest, state: &AppState) -> Result<Response, HttpError> {
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
