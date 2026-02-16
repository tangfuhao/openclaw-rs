use tracing::info;

/// Check if a newer version of OpenClaw is available.
pub async fn check_for_updates(current_version: &str) -> anyhow::Result<Option<String>> {
    let url = "https://api.github.com/repos/openclaw/openclaw-rs/releases/latest";

    let client = reqwest::Client::builder()
        .user_agent("openclaw-rs")
        .build()?;

    let resp = client.get(url).send().await?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let body: serde_json::Value = resp.json().await?;
    let latest_version = body["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v');

    if latest_version.is_empty() {
        return Ok(None);
    }

    if latest_version != current_version {
        info!("New version available: {latest_version} (current: {current_version})");
        Ok(Some(latest_version.to_string()))
    } else {
        Ok(None)
    }
}
