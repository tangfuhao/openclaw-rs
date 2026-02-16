/// Split text into overlapping chunks for embedding.
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if text.is_empty() || chunk_size == 0 {
        return Vec::new();
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= chunk_size {
        return vec![text.to_string()];
    }

    let step = chunk_size.saturating_sub(overlap).max(1);
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < words.len() {
        let end = (start + chunk_size).min(words.len());
        let chunk = words[start..end].join(" ");
        chunks.push(chunk);

        if end >= words.len() {
            break;
        }
        start += step;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunking() {
        let text = (0..100).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" ");
        let chunks = chunk_text(&text, 20, 5);
        assert!(chunks.len() > 1);
        assert!(chunks[0].split_whitespace().count() <= 20);
    }

    #[test]
    fn test_small_text() {
        let chunks = chunk_text("hello world", 100, 10);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn test_empty() {
        assert!(chunk_text("", 10, 2).is_empty());
    }
}
