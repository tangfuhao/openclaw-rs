use openclaw_config::ConfigManager;
use openclaw_infra::ports;

pub async fn run(config: ConfigManager) -> anyhow::Result<()> {
    println!("OpenClaw Doctor - System Diagnostics\n");

    // Check config
    print!("Configuration... ");
    let cfg = config.get();
    println!("OK (loaded from {})", config.config_path().display());

    // Check default port
    let port = cfg.gateway.port;
    print!("Port {port}... ");
    if ports::is_port_available(port) {
        println!("AVAILABLE");
    } else {
        println!("IN USE (gateway may be running)");
    }

    // Check providers
    print!("Model providers... ");
    if cfg.models.providers.is_empty() {
        println!("NONE CONFIGURED");
        println!("  Hint: Add provider API keys to your config file.");
    } else {
        let names: Vec<&String> = cfg.models.providers.keys().collect();
        println!("{} configured ({})", names.len(), names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
    }

    // Check channels
    print!("Channels... ");
    if cfg.channels.is_empty() {
        println!("NONE CONFIGURED");
    } else {
        let names: Vec<&String> = cfg.channels.keys().collect();
        println!("{} configured ({})", names.len(), names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
    }

    // Check memory
    print!("Memory/RAG... ");
    if cfg.memory.enabled {
        println!("ENABLED");
    } else {
        println!("DISABLED");
    }

    println!("\nAll checks completed.");
    Ok(())
}
