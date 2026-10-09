use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnterpriseAiPolicy {
    pub schema_version: u32,
    pub service_name: String,
    pub endpoint: String,
    pub models: Vec<String>,
    pub default_model: String,
}

impl EnterpriseAiPolicy {
    pub fn parse(json: &str) -> Result<Self, String> {
        let mut policy: Self =
            serde_json::from_str(json).map_err(|_| "Invalid enterprise AI configuration JSON or fields".to_string())?;
        if policy.schema_version != 1 {
            return Err("Enterprise AI schemaVersion must be 1".into());
        }
        policy.service_name = policy.service_name.trim().to_string();
        if policy.service_name.is_empty() {
            return Err("Enterprise AI serviceName is required".into());
        }
        let endpoint =
            url::Url::parse(policy.endpoint.trim()).map_err(|_| "Invalid enterprise AI endpoint".to_string())?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || endpoint.host_str().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(
                "Enterprise AI endpoint must be an HTTP(S) base URL without credentials, query or fragment".into()
            );
        }
        policy.endpoint = endpoint.to_string().trim_end_matches('/').to_string();
        let mut seen = HashSet::new();
        for model in &mut policy.models {
            *model = model.trim().to_string();
            if model.is_empty() || !seen.insert(model.clone()) {
                return Err("Enterprise AI models must be nonempty and unique".into());
            }
        }
        policy.default_model = policy.default_model.trim().to_string();
        if policy.models.is_empty() || !seen.contains(&policy.default_model) {
            return Err("Enterprise AI defaultModel must be in models".into());
        }
        Ok(policy)
    }
    pub fn validate_model(&self, model: &str) -> Result<(), String> {
        if self.models.iter().any(|allowed| allowed == model) {
            Ok(())
        } else {
            Err("ENTERPRISE_AI_MODEL_NOT_ALLOWED".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> serde_json::Value {
        serde_json::json!({"schemaVersion":1,"serviceName":"Company AI","endpoint":"https://AI.example/v1/","models":["a","b"],"defaultModel":"a"})
    }
    #[test]
    fn normalizes_and_validates_policy() {
        let p = EnterpriseAiPolicy::parse(&fixture().to_string()).unwrap();
        assert_eq!(p.endpoint, "https://ai.example/v1");
        assert!(p.validate_model("b").is_ok());
        assert!(p.validate_model("other").is_err());
    }
    #[test]
    fn rejects_invalid_fields_and_models() {
        for (field, value) in [
            ("schemaVersion", serde_json::json!(2)),
            ("serviceName", serde_json::json!(" ")),
            ("models", serde_json::json!([])),
            ("models", serde_json::json!(["a", " a "])),
            ("models", serde_json::json!([""])),
            ("defaultModel", serde_json::json!("missing")),
            ("apiKey", serde_json::json!("forbidden")),
        ] {
            let mut v = fixture();
            v[field] = value;
            assert!(EnterpriseAiPolicy::parse(&v.to_string()).is_err(), "{field}");
        }
        for field in ["schemaVersion", "serviceName", "endpoint", "models", "defaultModel"] {
            let mut v = fixture();
            v.as_object_mut().unwrap().remove(field);
            assert!(EnterpriseAiPolicy::parse(&v.to_string()).is_err());
        }
    }
    #[test]
    fn rejects_non_base_urls() {
        for endpoint in
            ["file:///tmp/key", "https://user:pass@host/v1", "https://host/v1?key=x", "https://host/v1#x", "not a url"]
        {
            let mut v = fixture();
            v["endpoint"] = serde_json::json!(endpoint);
            assert!(EnterpriseAiPolicy::parse(&v.to_string()).is_err());
        }
    }
}
