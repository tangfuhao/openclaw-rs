mod commands;
mod tui;

use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser)]
#[command(
    name = "openclaw",
    version = env!("CARGO_PKG_VERSION"),
    about = "OpenClaw - Multi-channel AI Gateway (Rust Edition)",
    long_about = "A high-performance multi-channel AI gateway that connects LLM agents to messaging platforms."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file path
    #[arg(short, long, global = true)]
    config: Option<String>,

    /// Verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the gateway server
    Gateway(commands::gateway::GatewayArgs),

    /// Interactive TUI chat interface
    Tui(commands::tui::TuiArgs),

    /// Configuration management
    Config(commands::config::ConfigArgs),

    /// Agent management
    Agent(commands::agent::AgentArgs),

    /// Session management
    Sessions(commands::sessions::SessionsArgs),

    /// Memory search and management
    Memory(commands::memory::MemoryArgs),

    /// Model configuration and listing
    Models(commands::models::ModelsArgs),

    /// Health and status checks
    Status(commands::status::StatusArgs),

    /// Run diagnostics
    Doctor,

    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Load .env
    openclaw_infra::dotenv::load_dotenv(&std::env::current_dir().unwrap_or_default());

    // Load config
    let config = if let Some(path) = &cli.config {
        openclaw_config::ConfigManager::load(path)?
    } else {
        openclaw_config::ConfigManager::load_default()?
    };

    match cli.command {
        Commands::Gateway(args) => commands::gateway::run(args, config).await,
        Commands::Tui(args) => commands::tui::run(args, config).await,
        Commands::Config(args) => commands::config::run(args, config).await,
        Commands::Agent(args) => commands::agent::run(args, config).await,
        Commands::Sessions(args) => commands::sessions::run(args, config).await,
        Commands::Memory(args) => commands::memory::run(args, config).await,
        Commands::Models(args) => commands::models::run(args, config).await,
        Commands::Status(args) => commands::status::run(args, config).await,
        Commands::Doctor => commands::doctor::run(config).await,
        Commands::Version => {
            println!("openclaw {} (rust)", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
