use regex::Regex;

/// Inline directives parsed from user messages.
#[derive(Debug, Clone)]
pub enum Directive {
    /// Override the model for this message.
    Model(String),
    /// Override the think level.
    ThinkLevel(String),
    /// Force a specific queue mode.
    QueueMode(String),
    /// Set temperature.
    Temperature(f32),
}

/// Parse inline directives from a message and return the clean text + directives.
pub fn parse_directives(text: &str) -> (String, Vec<Directive>) {
    let mut directives = Vec::new();
    let mut clean = text.to_string();

    // Pattern: /model:provider/model-name
    let model_re = Regex::new(r"\s*/model:(\S+)\s*").expect("valid regex");
    if let Some(caps) = model_re.captures(text) {
        directives.push(Directive::Model(caps[1].to_string()));
        clean = model_re.replace(&clean, " ").trim().to_string();
    }

    // Pattern: /think:level
    let think_re = Regex::new(r"\s*/think:(\S+)\s*").expect("valid regex");
    if let Some(caps) = think_re.captures(text) {
        directives.push(Directive::ThinkLevel(caps[1].to_string()));
        clean = think_re.replace(&clean, " ").trim().to_string();
    }

    // Pattern: /queue:mode
    let queue_re = Regex::new(r"\s*/queue:(\S+)\s*").expect("valid regex");
    if let Some(caps) = queue_re.captures(text) {
        directives.push(Directive::QueueMode(caps[1].to_string()));
        clean = queue_re.replace(&clean, " ").trim().to_string();
    }

    // Pattern: /temp:0.7
    let temp_re = Regex::new(r"\s*/temp:([\d.]+)\s*").expect("valid regex");
    if let Some(caps) = temp_re.captures(text) {
        if let Ok(temp) = caps[1].parse::<f32>() {
            directives.push(Directive::Temperature(temp));
        }
        clean = temp_re.replace(&clean, " ").trim().to_string();
    }

    (clean, directives)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_directive() {
        let (clean, directives) = parse_directives("Hello /model:anthropic/claude-3 world");
        assert_eq!(clean, "Hello world");
        assert!(matches!(&directives[0], Directive::Model(m) if m == "anthropic/claude-3"));
    }

    #[test]
    fn test_no_directives() {
        let (clean, directives) = parse_directives("Hello world");
        assert_eq!(clean, "Hello world");
        assert!(directives.is_empty());
    }

    #[test]
    fn test_multiple_directives() {
        let (_, directives) = parse_directives("/model:openai/gpt-4o /temp:0.5 Hello");
        assert_eq!(directives.len(), 2);
    }
}
