use openclaw_config::ConfigManager;
use std::time::Duration;

#[tokio::test]
async fn test_gateway_health_endpoint() {
    // Create a config with a random available port
    let port = openclaw_infra::ports::find_available_port(19000).unwrap();

    let config_json = format!(
        r#"{{ "gateway": {{ "port": {port} }} }}"#
    );

    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), &config_json).unwrap();

    let config = ConfigManager::load(tmp.path()).unwrap();

    // Start gateway in background
    let cfg_clone = config.clone();
    let handle = tokio::spawn(async move {
        openclaw_gateway::start_gateway_server(cfg_clone).await
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test health endpoint
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{port}/health"))
        .send()
        .await
        .expect("Health request should succeed");

    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    handle.abort();
}

#[tokio::test]
async fn test_openai_compat_endpoint() {
    let port = openclaw_infra::ports::find_available_port(19100).unwrap();

    let config_json = format!(r#"{{ "gateway": {{ "port": {port} }} }}"#);
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), &config_json).unwrap();

    let config = ConfigManager::load(tmp.path()).unwrap();
    let cfg_clone = config.clone();
    let handle = tokio::spawn(async move {
        openclaw_gateway::start_gateway_server(cfg_clone).await
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hello"}],
        }))
        .send()
        .await
        .expect("Chat completion request should succeed");

    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["object"], "chat.completion");
    assert!(!body["choices"].as_array().unwrap().is_empty());

    handle.abort();
}
