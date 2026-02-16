use clap::{Args, Subcommand};
use openclaw_config::ConfigManager;

#[derive(Args)]
pub struct AgentArgs {
    #[command(subcommand)]
    command: AgentCommand,
}

#[derive(Subcommand)]
enum AgentCommand {
    /// List configured agents
    List,
    /// Show agent details
    Show { name: String },
}

pub async fn run(args: AgentArgs, config: ConfigManager) -> anyhow::Result<()> {
    let cfg = config.get();
    match args.command {
        AgentCommand::List => {
            if cfg.agents.is_empty() {
                println!("No agents configured.");
            } else {
                println!("{:<20} {:<30} {}", "NAME", "MODEL", "DISPLAY NAME");
                for (name, agent) in &cfg.agents {
                    println!(
                        "{:<20} {:<30} {}",
                        name,
                        agent.model.as_deref().unwrap_or("-"),
                        agent.display_name.as_deref().unwrap_or("-"),
                    );
                }
            }
        }
        AgentCommand::Show { name } => {
            if let Some(agent) = cfg.agents.get(&name) {
                println!("{}", serde_json::to_string_pretty(agent)?);
            } else {
                println!("Agent '{name}' not found.");
            }
        }
    }
    Ok(())
}
