use openclaw_memory::chunker::chunk_text;
use openclaw_memory::search::cosine_similarity;

#[test]
fn test_chunk_text() {
    let text = (0..200).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" ");
    let chunks = chunk_text(&text, 50, 10);
    assert!(chunks.len() >= 4);
    assert!(chunks[0].split_whitespace().count() <= 50);
}

#[test]
fn test_cosine_similarity() {
    let a = vec![1.0, 0.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0, 0.0];
    assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

    let c = vec![0.0, 1.0, 0.0, 0.0];
    assert!(cosine_similarity(&a, &c).abs() < 1e-6);

    let d = vec![0.707, 0.707, 0.0, 0.0];
    let sim = cosine_similarity(&a, &d);
    assert!(sim > 0.5 && sim < 0.8);
}
