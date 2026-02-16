use regex::Regex;
use std::env;

/// Substitute `${VAR}` and `${VAR:-default}` patterns in a string
/// with environment variable values.
pub fn substitute_env_vars(input: &str) -> String {
    let re = Regex::new(r"\$\{([^}]+)\}").expect("valid regex");

    re.replace_all(input, |caps: &regex::Captures| {
        let expr = &caps[1];

        // Handle ${VAR:-default} syntax
        if let Some((var_name, default_val)) = expr.split_once(":-") {
            env::var(var_name.trim()).unwrap_or_else(|_| default_val.to_string())
        } else if let Some((var_name, default_val)) = expr.split_once("-") {
            // Handle ${VAR-default} syntax (only if unset, not if empty)
            match env::var(var_name.trim()) {
                Ok(val) => val,
                Err(_) => default_val.to_string(),
            }
        } else {
            env::var(expr.trim()).unwrap_or_default()
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_substitution() {
        // SAFETY: test runs single-threaded, no concurrent env access
        unsafe { env::set_var("TEST_OPENCLAW_VAR", "hello"); }
        let result = substitute_env_vars("value is ${TEST_OPENCLAW_VAR}");
        assert_eq!(result, "value is hello");
        unsafe { env::remove_var("TEST_OPENCLAW_VAR"); }
    }

    #[test]
    fn test_default_value() {
        let result = substitute_env_vars("${NONEXISTENT_VAR:-fallback}");
        assert_eq!(result, "fallback");
    }

    #[test]
    fn test_missing_var_empty() {
        let result = substitute_env_vars("prefix_${NONEXISTENT_VAR}_suffix");
        assert_eq!(result, "prefix__suffix");
    }
}
