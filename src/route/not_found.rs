use crate::domain::{AppState, NotFoundTemplate};
use crate::http::{HttpError, HttpRequest, Response};
use askama::Template;

pub fn not_found_handler(req: &HttpRequest, state: &AppState) -> Result<Response, HttpError> {
    let nav_items = state.generate_navigation(&req.path);
    let template = NotFoundTemplate {
        nav_items,
        current_page: req.path.clone(),
    };
    let html = template
        .render()
        .map_err(|e| HttpError::InternalServerError(e.to_string()))?;
    Ok(Response::not_found_with_html(html))
}
