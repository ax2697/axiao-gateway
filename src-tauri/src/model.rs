use serde::{Deserialize, Serialize};

fn default_format() -> String {
    "json".to_string()
}
fn default_true() -> bool {
    true
}
fn default_weight() -> f64 {
    1.0
}

/// 与 REA SetupClient 对齐的客户端规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientRule {
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub config_path: Option<serde_json::Value>,
    #[serde(default)]
    pub marker_path: Option<serde_json::Value>,
    #[serde(default)]
    pub skill_path: Option<serde_json::Value>,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub auth_file: Option<String>,
    #[serde(default)]
    pub api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub client: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub credits: Option<f64>,
    #[serde(default)]
    pub credit_usage: Option<f64>,
    #[serde(default)]
    pub credit_expiry: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub last_used_at: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    pub schema_version: String,
    #[serde(default)]
    pub exported_at: Option<String>,
    #[serde(default)]
    pub generator: Option<String>,
    pub client_rules: Vec<ClientRule>,
    pub accounts: Vec<Account>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Upstream {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub protocol: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_weight")]
    pub weight: f64,
    /// 该上游的默认模型（火山方舟要填 ep-xxxx 这种接入点 ID）
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    pub ts: String,
    pub account_id: String,
    pub upstream: String,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost: f64,
    /// HTTP 状态码（0 = 没拿到响应，如超时/连不上）
    #[serde(default)]
    pub status: u16,
    /// 端到端耗时（毫秒）
    #[serde(default)]
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEvent {
    pub ts: String,
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentStatus {
    pub id: String,
    pub name: String,
    pub role: String,
    pub path: String,
    pub exists: bool,
    pub detail: String,
    /// true = 能力已内置在本应用里，外部 exe 缺失也不影响使用
    #[serde(default)]
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub upstream: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub client_key: Option<String>,
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatResult {
    pub upstream: String,
    pub status: u16,
    pub data: serde_json::Value,
    pub cooldown: bool,
}
