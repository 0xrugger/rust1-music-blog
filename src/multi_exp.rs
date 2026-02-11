use anyhow::{Context, Result};
use multipart::server::Multipart;
use std::collections::HashMap;
use std::io::Read;
use std::time::Instant;
#[derive(Debug)]
pub enum FormField {
    Text(String),
    File {
        filename: String,
        content_type: String,
        data: Vec<u8>,
    },
}

pub fn parse_multipart(body: &[u8], boundary: &str) -> Result<HashMap<String, FormField>> {
    let (fields, _) = parse_multipart_with_metrics(body, boundary)?;
    Ok(fields)
}

pub fn parse_multipart_with_metrics<R: Read>(
    reader: R,
    boundary: &str,
) -> Result<(HashMap<String, FormField>, MultipartMetrics)> {
    let start = Instant::now();
    let mut metrics = MultipartMetrics::default();

    let mut multipart = Multipart::with_body(reader, boundary.to_string());
    let mut fields = HashMap::new();

    while let Some(mut entry) = multipart.read_entry()? {
        let name = entry.headers.name.clone();

        if let Some(filename) = entry.headers.filename {
            let content_type = entry
                .headers
                .content_type
                .as_ref()
                .map(|m| m.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            let mut data = Vec::new();
            entry.data.read_to_end(&mut data)?;

            metrics.file_count += 1;
            metrics.total_bytes += data.len();
            metrics.content_types.push(content_type.clone());

            fields.insert(
                name.to_string(),
                FormField::File {
                    filename,
                    content_type,
                    data,
                },
            );
        } else {
            let mut data = Vec::new();
            entry.data.read_to_end(&mut data)?;
            let text = String::from_utf8(data)?;
            metrics.text_field_count += 1;
            metrics.total_bytes += text.len();

            fields.insert(name.to_string(), FormField::Text(text));
        }
    }

    metrics.parse_duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok((fields, metrics))
}

#[derive(Default)]
pub struct MultipartMetrics {
    pub parse_duration_ms: f64,
    pub total_bytes: usize,
    pub file_count: usize,
    pub text_field_count: usize,
    pub content_types: Vec<String>,
}

pub fn extract_boundary(content_type: &str) -> Result<String> {
    for part in content_type.split(';') {
        let part = part.trim();
        if part.starts_with("boundary=") {
            let value = &part["boundary=".len()..];
            let clean = value.trim_matches('"').trim().to_string();
            if clean.is_empty() {
                anyhow::bail!("Empty boundary");
            }
            return Ok(clean);
        }
    }
    anyhow::bail!("No boundary found");
}
