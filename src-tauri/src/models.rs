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
