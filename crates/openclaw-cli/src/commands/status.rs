use clap::Args;
use openclaw_config::ConfigManager;

#[derive(Args)]
pub struct StatusArgs {
    /// Gateway URL to check
    #[arg(short, long, default_value = "http://localhost:18789")]
    url: String,
}

pub async fn run(args: StatusArgs, _config: ConfigManager) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    match client.get(format!("{}/health", args.url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp.json().await?;
            println!("Gateway Status: RUNNING");
            println!(
                "  Version:     {}",
                body["version"].as_str().unwrap_or("unknown")
            );
            println!(
                "  Uptime:      {}s",
                body["uptime_seconds"].as_i64().unwrap_or(0)
            );
            println!(
                "  Connections: {}",
                body["connections"].as_u64().unwrap_or(0)
            );
        }
        Ok(resp) => {
            println!("Gateway returned status: {}", resp.status());
        }
        Err(e) => {
            println!("Gateway Status: NOT RUNNING");
            println!("  Error: {e}");
            println!("  Start with: openclaw gateway");
        }
    }
    Ok(())
}
