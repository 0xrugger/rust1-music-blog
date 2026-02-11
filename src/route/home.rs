use crate::domain::{AppState, HomeTemplate};
use crate::http::{HttpError, HttpRequest, Response};
use askama::Template;

pub fn home_page_handler(req: &HttpRequest, state: &AppState) -> Result<Response, HttpError> {
    let show_upload_success = req
        .get_query_param("upload_success")
        .map(|s| s == "true")
        .unwrap_or(false);
    let nav_items = state.generate_navigation("/");
    let template = HomeTemplate {
        posts_count: state.posts.len(),
        nav_items,
        current_page: "/".to_string(),
        show_upload_success,
    };
    let html = template
        .render()
        .map_err(|e| HttpError::InternalServerError(e.to_string()))?;
    Ok(Response::html(200, html))
}
