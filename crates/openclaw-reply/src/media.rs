use openclaw_core::message::{MediaAttachment, MediaType};
use tracing::debug;

/// Process media attachments, extracting text content where applicable.
pub async fn process_media(media: &[MediaAttachment]) -> Vec<String> {
    let mut descriptions = Vec::new();

    for attachment in media {
        match &attachment.media_type {
            MediaType::Image => {
                if let Some(url) = &attachment.url {
                    descriptions.push(format!("[Image: {url}]"));
                } else {
                    descriptions.push("[Image attachment]".to_string());
                }
            }
            MediaType::Document => {
                if let Some(filename) = &attachment.filename {
                    descriptions.push(format!("[Document: {filename}]"));
                    // TODO: Extract text from PDF/DOCX using appropriate libraries
                }
            }
            MediaType::Audio | MediaType::Voice => {
                descriptions.push("[Audio attachment - transcription pending]".to_string());
                // TODO: Integrate with speech-to-text
            }
            MediaType::Video => {
                descriptions.push("[Video attachment]".to_string());
            }
            MediaType::Location => {
                descriptions.push("[Location shared]".to_string());
            }
            MediaType::Contact => {
                descriptions.push("[Contact shared]".to_string());
            }
            MediaType::Sticker => {
                descriptions.push("[Sticker]".to_string());
            }
        }
    }

    descriptions
}

/// Check if a MIME type is an image.
pub fn is_image_mime(mime: &str) -> bool {
    mime.starts_with("image/")
}

/// Check if a MIME type is processable.
pub fn is_processable_mime(mime: &str) -> bool {
    mime.starts_with("text/")
        || mime == "application/pdf"
        || mime == "application/json"
        || mime.contains("xml")
}
