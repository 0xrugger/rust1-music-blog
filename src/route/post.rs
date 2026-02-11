use crate::domain::{AppState, PostTemplate};
use crate::http::{HttpError, HttpRequest, Response};
use askama::Template;

pub fn post_page_handler(
    _request: &HttpRequest,
    slug: &str,
    state: &AppState,
) -> Result<Response, HttpError> {
    let post = state
        .find_post_by_slug(slug)
        .ok_or_else(|| HttpError::NotFound(format!("Post {} not found", slug)))?;

    let nav_items = state.generate_navigation(&format!("/post/{}", slug));
    let template = PostTemplate {
        post,
        nav_items,
        current_page: format!("/post/{}", slug),
    };

    let html = template
        .render()
        .map_err(|e| HttpError::InternalServerError(e.to_string()))?;
    Ok(Response::html(200, html))
}
