use openclaw_config::schema::AgentConfig;
use openclaw_core::types::AgentId;

/// Build the system prompt for an agent run.
pub fn build_system_prompt(config: &AgentConfig, _agent_id: &AgentId) -> String {
    let mut sections = Vec::new();

    // Identity section
    let display_name = config
        .display_name
        .as_deref()
        .unwrap_or("OpenClaw Assistant");
    sections.push(format!(
        "# Identity\nYou are {display_name}, an AI assistant powered by OpenClaw."
    ));

    // Custom system prompt if provided
    if let Some(custom_prompt) = &config.system_prompt {
        sections.push(format!("# Instructions\n{custom_prompt}"));
    }

    // Tool instructions
    sections.push(
        "# Tool Usage\n\
         You have access to tools that you can use to help answer questions and complete tasks. \
         When you need to use a tool, format your response according to the tool calling protocol. \
         Always explain your reasoning before using tools."
            .to_string(),
    );

    // Time context
    let now = chrono::Utc::now();
    sections.push(format!(
        "# Context\nCurrent date and time: {} UTC",
        now.format("%Y-%m-%d %H:%M:%S")
    ));

    // Guidelines
    sections.push(
        "# Guidelines\n\
         - Be helpful, accurate, and concise.\n\
         - If you're unsure about something, say so.\n\
         - Use tools when they would help provide better answers.\n\
         - Respect the user's privacy and preferences."
            .to_string(),
    );

    sections.join("\n\n")
}

/// Build a minimal prompt for subagents.
pub fn build_minimal_prompt(task_description: &str) -> String {
    format!(
        "You are a focused sub-agent. Complete the following task efficiently:\n\n{task_description}\n\n\
         Be concise and return only the relevant result."
    )
}
