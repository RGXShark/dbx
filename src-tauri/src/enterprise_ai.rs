//! Desktop-only policy. The compiled policy, never an IPC config, chooses the destination.
use crate::enterprise_ai_schema::EnterpriseAiPolicy;
use dbx_core::ai::{AiConfig, AiConfigItem, AiModelInfo};
use dbx_core::storage::Storage;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::LazyLock;
include!(concat!(env!("OUT_DIR"), "/enterprise_ai_config.rs"));

pub const CONFIG_ID: &str = "__enterprise_ai__";
static POLICY: LazyLock<Option<EnterpriseAiPolicy>> = LazyLock::new(|| {
    ENTERPRISE_AI_JSON.map(|json| EnterpriseAiPolicy::parse(json).expect("Build-validated enterprise AI policy"))
});
pub fn policy() -> Option<&'static EnterpriseAiPolicy> {
    POLICY.as_ref()
}
pub fn ensure_personal_mode() -> Result<(), String> {
    if policy().is_some() {
        Err("ENTERPRISE_AI_CONFIG_LOCKED".into())
    } else {
        Ok(())
    }
}
pub fn ensure_app_updates_allowed() -> Result<(), String> {
    if policy().is_some() {
        Err("ENTERPRISE_UPDATES_MANAGED_BY_IT".into())
    } else {
        Ok(())
    }
}
fn namespace(policy: &EnterpriseAiPolicy) -> String {
    format!("enterprise_ai.{:x}", Sha256::digest(policy.endpoint.as_bytes()))
}
// The global device namespace survives connection-list replacement. Prefixing each
// key with the endpoint hash keeps enterprise credentials separate from other globals.
const DEVICE_NAMESPACE: &str = "dbx.global";
async fn read_local_secret(
    storage: &Storage,
    policy: &EnterpriseAiPolicy,
    kind: &str,
) -> Result<Option<String>, String> {
    storage.get_secret(DEVICE_NAMESPACE, &format!("{}.{}", namespace(policy), kind)).await
}
async fn save_local_secret(
    storage: &Storage,
    policy: &EnterpriseAiPolicy,
    kind: &str,
    value: &str,
) -> Result<(), String> {
    storage.set_secret(DEVICE_NAMESPACE, &format!("{}.{}", namespace(policy), kind), value).await
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseAiState {
    #[serde(flatten)]
    pub policy: EnterpriseAiPolicy,
    pub selected_model: String,
    pub has_api_key: bool,
}
pub async fn state(storage: &Storage) -> Result<Option<EnterpriseAiState>, String> {
    let Some(policy) = policy() else {
        return Ok(None);
    };
    state_for(storage, policy).await.map(Some)
}
async fn state_for(storage: &Storage, policy: &EnterpriseAiPolicy) -> Result<EnterpriseAiState, String> {
    let selected = read_local_secret(storage, policy, "selected_model")
        .await?
        .filter(|model| policy.validate_model(model).is_ok())
        .unwrap_or_else(|| policy.default_model.clone());
    let has_api_key = read_local_secret(storage, policy, "api_key").await?.is_some_and(|key| !key.trim().is_empty());
    Ok(EnterpriseAiState { policy: policy.clone(), selected_model: selected, has_api_key })
}
pub async fn save_key(storage: &Storage, key: &str) -> Result<(), String> {
    let policy = policy().ok_or("ENTERPRISE_AI_NOT_ENABLED")?;
    // These keys are neither AI config records nor sync-password records; sync excludes them.
    save_local_secret(storage, policy, "api_key", key.trim()).await
}
pub async fn save_model(storage: &Storage, model: &str) -> Result<(), String> {
    let policy = policy().ok_or("ENTERPRISE_AI_NOT_ENABLED")?;
    policy.validate_model(model)?;
    save_local_secret(storage, policy, "selected_model", model).await
}
fn config_for(policy: &EnterpriseAiPolicy, model: &str, key: &str) -> Result<AiConfig, String> {
    policy.validate_model(model)?;
    let mut config: AiConfig = serde_json::from_value(serde_json::json!({
        "provider":"openai-compatible", "apiStyle":"completions", "authMethod":"bearer",
        "endpoint":policy.endpoint, "model":model, "apiKey":key,
        "models":policy.models.iter().map(|name| serde_json::json!({"name":name})).collect::<Vec<_>>()
    }))
    .map_err(|error| error.to_string())?;
    config.deny_redirects = true;
    Ok(config)
}
pub async fn apply(storage: &Storage, submitted: AiConfig) -> Result<AiConfig, String> {
    let Some(policy) = policy() else {
        return Ok(submitted);
    };
    apply_for(storage, policy, submitted).await
}
async fn apply_for(storage: &Storage, policy: &EnterpriseAiPolicy, submitted: AiConfig) -> Result<AiConfig, String> {
    policy.validate_model(&submitted.model)?;
    let key = read_local_secret(storage, policy, "api_key")
        .await?
        .filter(|key| !key.is_empty())
        .ok_or("ENTERPRISE_AI_KEY_REQUIRED")?;
    config_for(policy, &submitted.model, &key)
}
pub async fn public_config(storage: &Storage) -> Result<Option<AiConfigItem>, String> {
    let Some(state) = state(storage).await? else {
        return Ok(None);
    };
    Ok(Some(AiConfigItem {
        id: CONFIG_ID.into(),
        name: state.policy.service_name.clone(),
        is_default: true,
        config: config_for(&state.policy, &state.selected_model, "")?,
    }))
}
pub fn models() -> Option<Vec<AiModelInfo>> {
    policy().map(|policy| policy.models.iter().map(|id| AiModelInfo::new(id, None)).collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn policy_for(endpoint: &str) -> EnterpriseAiPolicy {
        EnterpriseAiPolicy::parse(&serde_json::json!({"schemaVersion":1,"serviceName":"AI","endpoint":endpoint,"models":["a"],"defaultModel":"a"}).to_string()).unwrap()
    }
    #[test]
    fn effective_config_has_only_company_transport() {
        let p = policy_for("https://company.example/v1");
        let c = config_for(&p, "a", "personal-secret").unwrap();
        assert_eq!(c.endpoint, p.endpoint);
        assert_eq!(c.api_key, "personal-secret");
        assert!(c.deny_redirects);
        assert!(!c.proxy_enabled);
        assert!(!c.skip_tls_verify);
        assert!(c.custom_headers.is_empty());
        assert!(c.codex_cli_env.is_empty());
        assert!(!dbx_core::ai::is_cli_provider(&c.provider));
        assert!(config_for(&p, "other", "key").is_err());
        assert!(serde_json::to_value(c).unwrap().get("denyRedirects").is_none());
    }
    #[test]
    fn credentials_follow_endpoint_not_model_catalog() {
        let mut p = policy_for("https://company.example/v1");
        let original = namespace(&p);
        p.models.push("b".into());
        assert_eq!(namespace(&p), original);
        assert_ne!(namespace(&policy_for("https://other.example/v1")), original);
    }
    #[tokio::test]
    async fn personal_credentials_are_isolated_and_survive_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dbx.db");
        let storage = dbx_core::persistence::test_storage::open(&path).await.unwrap();
        let p = policy_for("https://company.example/v1");
        let legacy = config_for(&p, "a", "legacy-key").unwrap();
        storage
            .save_ai_config_item(&AiConfigItem {
                id: "old-provider".into(),
                name: "Old".into(),
                is_default: true,
                config: legacy,
            })
            .await
            .unwrap();
        assert!(!state_for(&storage, &p).await.unwrap().has_api_key);
        let mut submitted = config_for(&p, "a", "forged-key").unwrap();
        submitted.endpoint = "https://evil.example/v1".into();
        submitted.custom_headers.insert("Authorization".into(), "Bearer forged".into());
        submitted.proxy_enabled = true;
        submitted.proxy_url = "http://evil.example:8080".into();
        submitted.skip_tls_verify = true;
        submitted.provider = dbx_core::ai::AiProvider::CodexCli;
        submitted.codex_cli_path = Some("should-never-execute".into());
        assert!(apply_for(&storage, &p, submitted.clone()).await.is_err());
        save_local_secret(&storage, &p, "api_key", "employee-key").await.unwrap();
        let effective = apply_for(&storage, &p, submitted).await.unwrap();
        assert_eq!(effective.endpoint, p.endpoint);
        assert_eq!(effective.api_key, "employee-key");
        assert!(effective.custom_headers.is_empty());
        assert!(!effective.proxy_enabled);
        assert!(!effective.skip_tls_verify);
        assert!(effective.codex_cli_path.is_none());
        assert!(!dbx_core::ai::is_cli_provider(&effective.provider));
        let public = serde_json::to_string(&state_for(&storage, &p).await.unwrap()).unwrap();
        assert!(!public.contains("employee-key"));
        assert!(!state_for(&storage, &policy_for("https://other.example/v1")).await.unwrap().has_api_key);
        save_local_secret(&storage, &p, "selected_model", "removed-model").await.unwrap();
        assert_eq!(state_for(&storage, &p).await.unwrap().selected_model, "a");
        assert_eq!(storage.load_ai_configs().await.unwrap()[0].config.api_key, "legacy-key");
        let snapshot = dbx_core::cloud_sync::build_sync_snapshot(&storage, "test", None, Some("fixture-passphrase"))
            .await
            .unwrap();
        let target_dir = tempfile::tempdir().unwrap();
        let target = dbx_core::persistence::test_storage::open(&target_dir.path().join("dbx.db")).await.unwrap();
        dbx_core::cloud_sync::apply_sync_snapshot(
            &target,
            &snapshot,
            dbx_core::cloud_sync::ApplySnapshotOptions {
                secrets_passphrase: Some("fixture-passphrase"),
                restore_secrets: true,
            },
        )
        .await
        .unwrap();
        assert!(read_local_secret(&target, &p, "api_key").await.unwrap().is_none());
        dbx_core::cloud_sync::apply_sync_snapshot(
            &storage,
            &snapshot,
            dbx_core::cloud_sync::ApplySnapshotOptions {
                secrets_passphrase: Some("fixture-passphrase"),
                restore_secrets: true,
            },
        )
        .await
        .unwrap();
        assert_eq!(read_local_secret(&storage, &p, "api_key").await.unwrap().as_deref(), Some("employee-key"));
        assert_eq!(target.load_ai_configs().await.unwrap()[0].config.api_key, "legacy-key");
        drop(storage);
        assert!(!std::fs::read(&path).unwrap().windows(b"employee-key".len()).any(|bytes| bytes == b"employee-key"));
        let storage = dbx_core::persistence::test_storage::open(&path).await.unwrap();
        assert_eq!(read_local_secret(&storage, &p, "api_key").await.unwrap().as_deref(), Some("employee-key"));
        save_local_secret(&storage, &p, "api_key", "replacement").await.unwrap();
        assert_eq!(read_local_secret(&storage, &p, "api_key").await.unwrap().as_deref(), Some("replacement"));
        save_local_secret(&storage, &p, "api_key", "").await.unwrap();
        assert!(!state_for(&storage, &p).await.unwrap().has_api_key);
    }
    #[tokio::test]
    async fn requests_cannot_redirect_to_another_service() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0; 4096];
            let count = stream.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..count]).to_lowercase();
            assert!(request.contains("authorization: bearer employee-key"));
            stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
        });
        let config = config_for(&policy_for(&endpoint), "a", "employee-key").unwrap();
        let client = dbx_core::ai::build_ai_http_client(&config, 2).unwrap();
        let response = client
            .get(format!("{endpoint}/models"))
            .headers(dbx_core::ai::maybe_bearer_headers(&config).unwrap())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 302);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn completion_stream_test_and_agent_transport_use_company_credentials() {
        use axum::{
            response::{IntoResponse, Response},
            routing::post,
            Json, Router,
        };
        use std::sync::{Arc, Mutex};
        let captured = Arc::new(Mutex::new(Vec::new()));
        let capture = captured.clone();
        type Captured = Arc<Mutex<Vec<(axum::http::HeaderMap, serde_json::Value)>>>;
        async fn completion_handler(
            axum::extract::State(capture): axum::extract::State<Captured>,
            headers: axum::http::HeaderMap,
            Json(body): Json<serde_json::Value>,
        ) -> Response {
            let authorized = headers.get("authorization").is_some_and(|value| value == "Bearer employee-key");
            capture.lock().unwrap().push((headers, body.clone()));
            if !authorized {
                return (
                    axum::http::StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error":{"message":"key revoked"}})),
                )
                    .into_response();
            }
            if body["stream"].as_bool() == Some(true) {
                let delta = serde_json::json!({"choices":[{"index":0,"delta":{"content":"ok"},"finish_reason":null}]});
                let stop = serde_json::json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]});
                ([("content-type", "text/event-stream")], format!("data: {delta}\n\ndata: {stop}\n\ndata: [DONE]\n\n"))
                    .into_response()
            } else {
                Json(serde_json::json!({"choices":[{"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}]})).into_response()
            }
        }
        let app = Router::new().route("/v1/chat/completions", post(completion_handler)).with_state(capture);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let dir = tempfile::tempdir().unwrap();
        let storage = dbx_core::persistence::test_storage::open(&dir.path().join("dbx.db")).await.unwrap();
        let p = policy_for(&endpoint);
        save_local_secret(&storage, &p, "api_key", "employee-key").await.unwrap();
        let mut forged = config_for(&p, "a", "forged").unwrap();
        forged.endpoint = "https://evil.example".into();
        forged.provider = dbx_core::ai::AiProvider::CodexCli;
        let config = apply_for(&storage, &p, forged).await.unwrap();
        let request: dbx_core::ai::AiCompletionRequest = serde_json::from_value(
            serde_json::json!({"config":config,"systemPrompt":"","messages":[{"role":"user","content":"hello"}]}),
        )
        .unwrap();
        // The flag is transport-only and excluded from serde, so build the actual runtime request with its authoritative config.
        let mut request = request;
        request.config = config;
        assert_eq!(dbx_core::ai::complete(&request).await.unwrap(), "ok");
        let cancel = tokio::sync::Notify::new();
        dbx_core::ai::stream("enterprise-test", &request, &cancel, |_| {}).await.unwrap();
        dbx_core::ai::stream_with_tools(&request.config, &request, "enterprise-agent-test", &[], &cancel, |_| {})
            .await
            .unwrap();
        assert!(dbx_core::ai::test_connection_core(&request.config).await.unwrap().success);
        {
            let records = captured.lock().unwrap();
            assert_eq!(records.len(), 4);
            for (headers, body) in records.iter() {
                assert_eq!(headers["authorization"], "Bearer employee-key");
                assert_eq!(body["model"], "a");
            }
        }
        save_local_secret(&storage, &p, "api_key", "revoked-key").await.unwrap();
        let rejected = apply_for(&storage, &p, request.config.clone()).await.unwrap();
        let error = dbx_core::ai::test_connection_core(&rejected).await.unwrap_err();
        assert!(error.starts_with("[auth]"));
        server.abort();
    }
}
