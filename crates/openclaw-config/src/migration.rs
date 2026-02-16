use serde_json::Value;
use tracing::{info, warn};

/// Migrate legacy configuration formats to the current schema.
/// Returns true if any migration was applied.
pub fn migrate_config(config: &mut Value) -> bool {
    let mut migrated = false;

    if let Some(obj) = config.as_object_mut() {
        // Migration: "daemon" -> "gateway" rename
        if obj.contains_key("daemon") && !obj.contains_key("gateway") {
            if let Some(daemon_val) = obj.remove("daemon") {
                info!("Migrating 'daemon' config key to 'gateway'");
                obj.insert("gateway".to_string(), daemon_val);
                migrated = true;
            }
        }

        // Migration: "provider" (singular) -> "models.providers"
        if obj.contains_key("provider") && !obj.contains_key("models") {
            if let Some(provider_val) = obj.remove("provider") {
                info!("Migrating 'provider' config to 'models.providers'");
                let mut models = serde_json::Map::new();
                let mut providers = serde_json::Map::new();
                if let Some(prov_obj) = provider_val.as_object() {
                    for (k, v) in prov_obj {
                        providers.insert(k.clone(), v.clone());
                    }
                }
                models.insert("providers".to_string(), Value::Object(providers));
                obj.insert("models".to_string(), Value::Object(models));
                migrated = true;
            }
        }

        // Migration: "autoReply" -> "agents.default"
        if obj.contains_key("autoReply") && !obj.contains_key("agents") {
            if let Some(auto_reply) = obj.remove("autoReply") {
                info!("Migrating 'autoReply' config to 'agents.default'");
                let mut agents = serde_json::Map::new();
                agents.insert("default".to_string(), auto_reply);
                obj.insert("agents".to_string(), Value::Object(agents));
                migrated = true;
            }
        }

        // Migration: flatten "extensions" into "channels"
        if obj.contains_key("extensions") && !obj.contains_key("channels") {
            if let Some(extensions) = obj.remove("extensions") {
                info!("Migrating 'extensions' config to 'channels'");
                obj.insert("channels".to_string(), extensions);
                migrated = true;
            }
        }
    }

    if migrated {
        warn!("Configuration was migrated from a legacy format. Please update your config file.");
    }

    migrated
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_daemon_to_gateway_migration() {
        let mut config = json!({
            "daemon": { "port": 8080 }
        });
        assert!(migrate_config(&mut config));
        assert!(config.get("gateway").is_some());
        assert!(config.get("daemon").is_none());
        assert_eq!(config["gateway"]["port"], 8080);
    }

    #[test]
    fn test_no_migration_needed() {
        let mut config = json!({
            "gateway": { "port": 18789 }
        });
        assert!(!migrate_config(&mut config));
    }
}
