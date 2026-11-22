use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteRecord {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub status: SiteStatus,
    pub php_version: String,
    pub wordpress_version: String,
    pub database_mode: DatabaseMode,
    pub port: u16,
    pub domain: String,
    pub path: String,
    pub provisioned: bool,
    pub created_at: String,
    #[serde(default)]
    pub https_enabled: bool,
    #[serde(default)]
    pub custom_domain: Option<String>,
    #[serde(default)]
    pub last_snapshot_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SiteStatus { Stopped, Running, Provisioning, NeedsRuntime, Error }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseMode { Sqlite, Mariadb }

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSiteInput {
    pub name: String,
    pub slug: String,
    pub php_version: String,
    pub wordpress_version: String,
    pub database_mode: DatabaseMode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolProbe {
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePreflight {
    pub app_data_dir: String,
    pub runtime_pack_dir: String,
    pub cached_wordpress: bool,
    pub cached_sqlite_integration: bool,
    pub cached_mariadb: bool,
    pub cached_cloudflared: bool,
    pub cached_caddy: bool,
    pub cached_mailpit: bool,
    pub tools: Vec<ToolProbe>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionResult {
    pub site: SiteRecord,
    pub site_url: String,
    pub admin_user: String,
    pub admin_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseCredentials {
    pub database: String,
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseResult { pub output: String, pub rows_affected: Option<u64> }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRecord {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub path: String,
    pub size_bytes: u64,
    pub database_mode: DatabaseMode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRecord {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub available: bool,
    pub initialized: bool,
    pub branch: Option<String>,
    pub dirty: bool,
    pub output: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareInfo {
    pub running: bool,
    pub url: Option<String>,
    pub log_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterInfo {
    pub enabled: bool,
    pub url: Option<String>,
    pub domain: Option<String>,
    pub config_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployInput {
    pub site_id: String,
    pub kind: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub secret: String,
    pub remote_path: String,
    pub site_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployResult {
    pub success: bool,
    pub stage: String,
    pub message: String,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityReport {
    pub php_version: String,
    pub wordpress_version: String,
    pub core_checksums_ok: bool,
    pub plugin_count: usize,
    pub theme_count: usize,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVersionSource {
    pub key: String,
    pub version: String,
    pub url: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVersionItem {
    pub key: String,
    pub version: String,
    pub installed: bool,
    pub bundled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVersionCatalog {
    pub php: Vec<RuntimeVersionItem>,
    pub wordpress: Vec<RuntimeVersionItem>,
}
