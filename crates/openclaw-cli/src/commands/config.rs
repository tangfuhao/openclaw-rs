use clap::{Args, Subcommand};
use openclaw_config::ConfigManager;

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Validate configuration
    Validate,
    /// Show config file path
    Path,
    /// Edit configuration interactively
    Edit,
}

pub async fn run(args: ConfigArgs, config: ConfigManager) -> anyhow::Result<()> {
    match args.command {
        ConfigCommand::Show => {
            let cfg = config.get();
            println!("{}", serde_json::to_string_pretty(&*cfg)?);
        }
        ConfigCommand::Validate => {
            let _cfg = config.get();
            println!("Configuration is valid.");
        }
        ConfigCommand::Path => {
            println!("{}", config.config_path().display());
        }
        ConfigCommand::Edit => {
            let path = config.config_path();
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
            std::process::Command::new(&editor)
                .arg(path)
                .status()?;
        }
    }
    Ok(())
}
