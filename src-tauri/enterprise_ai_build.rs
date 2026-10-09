//! Shared by build.rs and desktop tests so embedding is tested without invoking the bundler.
use crate::enterprise_ai_schema::EnterpriseAiPolicy;
use std::path::Path;

pub fn embed(path: Option<&Path>, out: &Path) -> Result<(), String> {
    let json = match path {
        None => None,
        Some(path) => {
            if path.as_os_str().is_empty() {
                return Err("DBX_ENTERPRISE_AI_CONFIG must name a configuration file".into());
            }
            let source =
                std::fs::read_to_string(path).map_err(|_| "Cannot read enterprise AI configuration".to_string())?;
            let policy = EnterpriseAiPolicy::parse(&source)?;
            Some(serde_json::to_string(&policy).map_err(|error| error.to_string())?)
        }
    };
    let embedded = match json {
        Some(json) => format!("Some({json:?})"),
        None => "None".into(),
    };
    std::fs::write(
        out.join("enterprise_ai_config.rs"),
        format!("pub const ENTERPRISE_AI_JSON: Option<&str> = {embedded};\n"),
    )
    .map_err(|error| error.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_invalid_and_changed_configuration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("policy.json");
        assert!(embed(Some(&path), dir.path()).is_err());
        std::fs::write(&path, r#"{"apiKey":"forbidden"}"#).unwrap();
        assert!(embed(Some(&path), dir.path()).is_err());
        let mut policy = serde_json::json!({"schemaVersion":1,"serviceName":"AI","endpoint":"https://one.example/v1","models":["a"],"defaultModel":"a"});
        std::fs::write(&path, policy.to_string()).unwrap();
        embed(Some(&path), dir.path()).unwrap();
        let first = std::fs::read_to_string(dir.path().join("enterprise_ai_config.rs")).unwrap();
        assert!(first.contains("one.example"));
        policy["endpoint"] = serde_json::json!("https://two.example/v1");
        std::fs::write(&path, policy.to_string()).unwrap();
        embed(Some(&path), dir.path()).unwrap();
        let second = std::fs::read_to_string(dir.path().join("enterprise_ai_config.rs")).unwrap();
        assert!(second.contains("two.example"));
        assert!(!second.contains("one.example"));
        embed(None, dir.path()).unwrap();
        assert!(std::fs::read_to_string(dir.path().join("enterprise_ai_config.rs")).unwrap().contains("= None;"));
    }
}
