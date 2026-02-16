use clap::{Args, Subcommand};
use openclaw_config::ConfigManager;

#[derive(Args)]
pub struct ModelsArgs {
    #[command(subcommand)]
    command: ModelsCommand,
}

#[derive(Subcommand)]
enum ModelsCommand {
    /// List available models
    List,
    /// Show model aliases
    Aliases,
}

pub async fn run(args: ModelsArgs, config: ConfigManager) -> anyhow::Result<()> {
    let cfg = config.get();
    match args.command {
        ModelsCommand::List => {
            println!("{:<40} {}", "MODEL", "PROVIDER");
            for (provider, pc) in &cfg.models.providers {
                for model in &pc.models {
                    println!("{:<40} {}", format!("{provider}/{model}"), provider);
                }
            }
            if cfg.models.providers.is_empty() {
                println!("No models configured. Add providers to your config file.");
            }
        }
        ModelsCommand::Aliases => {
            if cfg.models.aliases.is_empty() {
                println!("No model aliases configured.");
            } else {
                println!("{:<20} {}", "ALIAS", "MODEL");
                for (alias, model) in &cfg.models.aliases {
                    println!("{:<20} {}", alias, model);
                }
            }
        }
    }
    Ok(())
}
