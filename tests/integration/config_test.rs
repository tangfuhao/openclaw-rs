use openclaw_config::ConfigManager;

#[test]
fn test_default_config_loading() {
    // Default config should work even without a file
    // (will use built-in defaults if file doesn't exist)
    let config = openclaw_config::schema::OpenClawConfig::default();
    assert_eq!(config.gateway.port, 18789);
    assert!(config.memory.enabled);
    assert_eq!(config.sessions.compaction_threshold, 50);
}

#[test]
fn test_config_from_json5() {
    let json5 = r#"{
        gateway: { port: 9999 },
        models: {
            defaultModel: "openai/gpt-4o",
            providers: {
                openai: {
                    apiKey: "test-key",
                    models: ["gpt-4o"],
                },
            },
        },
    }"#;

    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), json5).unwrap();

    let config = ConfigManager::load(tmp.path()).unwrap();
    let cfg = config.get();

    assert_eq!(cfg.gateway.port, 9999);
    assert_eq!(cfg.models.default_model.as_deref(), Some("openai/gpt-4o"));
    assert!(cfg.models.providers.contains_key("openai"));
}

#[test]
fn test_env_substitution() {
    // SAFETY: test runs single-threaded
    unsafe { std::env::set_var("TEST_OC_KEY", "secret123"); }

    let json5 = r#"{ gateway: { authToken: "${TEST_OC_KEY}" } }"#;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), json5).unwrap();

    let config = ConfigManager::load(tmp.path()).unwrap();
    let cfg = config.get();
    assert_eq!(cfg.gateway.auth_token.as_deref(), Some("secret123"));

    unsafe { std::env::remove_var("TEST_OC_KEY"); }
}
