use crate::http::HttpError;
use askama::Template;

// ============= SLUG UTILS =============
pub fn slugify(text: &str) -> String {
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

pub fn validate_slug(slug: &str) -> Result<(), HttpError> {
    if slug.is_empty() {
        return Err(HttpError::ValidationError("Slug empty".to_string()));
    }
    if slug.len() < 3 || slug.len() > 50 {
        return Err(HttpError::ValidationError("Slug length error".to_string()));
    }
    if slug.starts_with('-') || slug.ends_with('-') {
        return Err(HttpError::ValidationError(
            "Slug start/end contains '-'".to_string(),
        ));
    }
    if slug.contains("--") {
        return Err(HttpError::ValidationError("Slug contains '--'".to_string()));
    }
    for ch in slug.chars() {
        if !(ch.is_ascii_alphanumeric() || ch == '-') {
            return Err(HttpError::ValidationError(
                "Slug grammar is not correct".to_string(),
            ));
        }
    }
    Ok(())
}

// ============= TEMPLATES =============
#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    pub posts_count: usize,
    pub nav_items: Vec<NavItem>,
    pub current_page: String,
    pub show_upload_success: bool,
}

#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFoundTemplate {
    pub nav_items: Vec<NavItem>,
    pub current_page: String,
}

#[derive(Template)]
#[template(path = "post.html")]
pub struct PostTemplate<'a> {
    pub post: &'a Post,
    pub nav_items: Vec<NavItem>,
    pub current_page: String,
}

// ============= DOMAIN MODELS =============
pub struct Post {
    pub id: u32,
    pub slug: String,
    pub text: String,
    pub title: String,
    pub filename: Option<String>,
    pub file_data: Option<Vec<u8>>,
}

pub struct NavItem {
    pub title: String,
    pub url: String,
    pub is_current: bool,
}

pub struct AppState {
    pub posts: Vec<Post>,
}

impl AppState {
    pub fn new() -> Self {
        Self { posts: Vec::new() }
    }

    pub fn find_post_by_slug(&self, slug: &str) -> Option<&Post> {
        self.posts.iter().find(|post| post.slug == slug)
    }

    pub fn generate_navigation(&self, current_path: &str) -> Vec<NavItem> {
        self.posts
            .iter()
            .enumerate()
            .map(|(index, post)| {
                let url = format!("/post/{}", post.slug);
                NavItem {
                    title: format!("POST {}", index + 1),
                    url: url.clone(),
                    is_current: current_path == url,
                }
            })
            .collect()
    }
}
