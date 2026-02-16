use clap::Args;
use openclaw_config::ConfigManager;

#[derive(Args)]
pub struct TuiArgs {
    /// Gateway URL to connect to
    #[arg(short, long, default_value = "ws://localhost:18789/ws")]
    url: String,

    /// Session key
    #[arg(short, long)]
    session: Option<String>,
}

pub async fn run(args: TuiArgs, _config: ConfigManager) -> anyhow::Result<()> {
    println!("Starting OpenClaw TUI...");
    println!("Connecting to: {}", args.url);

    // TODO: Launch ratatui-based TUI
    crate::tui::chat::run_chat_tui(&args.url, args.session.as_deref()).await
}
