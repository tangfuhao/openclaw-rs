use clap::Args;
use openclaw_config::ConfigManager;
use tracing::info;

#[derive(Args)]
pub struct GatewayArgs {
    /// Port to listen on (overrides config)
    #[arg(short, long)]
    port: Option<u16>,

    /// Host to bind to
    #[arg(long)]
    host: Option<String>,

    /// Allow starting without configuration
    #[arg(long)]
    allow_unconfigured: bool,
}

pub async fn run(_args: GatewayArgs, config: ConfigManager) -> anyhow::Result<()> {
    info!("Starting OpenClaw gateway...");

    // Apply CLI overrides
    // TODO: Apply port/host overrides to config

    openclaw_gateway::start_gateway_server(config).await
}
