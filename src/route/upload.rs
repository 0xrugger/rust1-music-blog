use std::collections::HashMap;

use crate::domain::{AppState, Post, slugify, validate_slug};
use crate::http::{HttpError, HttpRequest, Response};
use crate::multi_exp::FormField;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

pub fn upload_post_handler(req: &HttpRequest, state: &mut AppState) -> Result<Response, HttpError> {
    match req.content_type.as_deref() {
        Some(ct) if ct.starts_with("multipart/form-data") => upload_multipart_handler(req, state),
        Some(ct) if ct == "application/x-www-form-urlencoded" => {
            upload_urlencoded_handler(req, state)
        }
        Some(ct) => Err(HttpError::BadRequest(format!(
            "Unsupported content type: {}",
            ct
        ))),
        None => Err(HttpError::BadRequest(
            "Missing Content-Type header".to_string(),
        )),
    }
}

fn upload_urlencoded_handler(
    req: &HttpRequest,
    state: &mut AppState,
) -> Result<Response, HttpError> {
    let form_data = HttpRequest::parse_urlencoded(&req.body)
        .map_err(|_| HttpError::BadRequest("URL Encoding error".to_string()))?;

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
        id,
        slug,
        text,
        title,
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

fn extract_text_field(
    fields: &HashMap<String, FormField>,
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

fn validate_image(data: &[u8], filename: &str) -> Result<(), HttpError> {
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
            "Incorrect file extension: {}. Try: {}",
            extension,
            allowed_extensions.join(", ")
        )));
    }

    if data.len() < 8 {
        return Err(HttpError::BadRequest(
            "File too short to validate".to_string(),
        ));
    }

    match extension.as_str() {
        "png" => {
            let png_signature: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
            if data[0..8] != png_signature {
                return Err(HttpError::BadRequest("Incorrect PNG signature".to_string()));
            }
        }
        "jpg" | "jpeg" => {
            let jpeg_signature: [u8; 2] = [255, 216];
            if data[0..2] != jpeg_signature {
                return Err(HttpError::BadRequest(
                    "Incorrect JPEG signature".to_string(),
                ));
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
