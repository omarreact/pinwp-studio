export type DatabaseMode = 'sqlite' | 'mariadb';
export type SiteStatus = 'stopped' | 'running' | 'provisioning' | 'needsRuntime' | 'error';

export interface SiteRecord {
  id: string;
  name: string;
  slug: string;
  status: SiteStatus;
  phpVersion: string;
  wordpressVersion: string;
  databaseMode: DatabaseMode;
  port: number;
  domain: string;
  path: string;
  provisioned: boolean;
  createdAt: string;
  httpsEnabled: boolean;
  customDomain?: string | null;
  lastSnapshotAt?: string | null;
}

export interface CreateSiteInput {
  name: string;
  slug: string;
  phpVersion: string;
  wordpressVersion: string;
  databaseMode: DatabaseMode;
}

export interface ToolProbe {
  name: string;
  available: boolean;
  version?: string | null;
  source?: string | null;
}

export interface RuntimePreflight {
  appDataDir: string;
  runtimePackDir: string;
  cachedWordpress: boolean;
  cachedSqliteIntegration: boolean;
  cachedMariadb: boolean;
  cachedCloudflared: boolean;
  cachedCaddy: boolean;
  cachedMailpit: boolean;
  tools: ToolProbe[];
}

export interface ProvisionResult {
  site: SiteRecord;
  siteUrl: string;
  adminUser: string;
  adminPassword: string;
}

export interface DatabaseResult { output: string; rowsAffected?: number | null; }
export interface SnapshotRecord {
  id: string;
  name: string;
  createdAt: string;
  path: string;
  sizeBytes: number;
  databaseMode: DatabaseMode;
}
export interface LogRecord { name: string; path: string; sizeBytes: number; modifiedAt?: string | null; }
export interface GitInfo { available: boolean; initialized: boolean; branch?: string | null; dirty: boolean; output: string; }
export interface ShareInfo { running: boolean; url?: string | null; logPath?: string | null; }
export interface RouterInfo { enabled: boolean; url?: string | null; domain?: string | null; configPath?: string | null; }
export interface DeployInput {
  siteId: string;
  kind: 'ssh' | 'cpanel' | 'connector';
  host: string;
  port?: number | null;
  username: string;
  secret: string;
  remotePath: string;
  siteUrl: string;
}
export interface DeployResult { success: boolean; stage: string; message: string; artifactPath?: string | null; }
export interface CompatibilityReport {
  phpVersion: string;
  wordpressVersion: string;
  coreChecksumsOk: boolean;
  pluginCount: number;
  themeCount: number;
  output: string;
}

export interface RuntimeVersionItem {
  key: string;
  version: string;
  installed: boolean;
  bundled: boolean;
}
export interface RuntimeVersionCatalog {
  php: RuntimeVersionItem[];
  wordpress: RuntimeVersionItem[];
}

export interface CompatibilityMatrixRow {
  phpVersion: string;
  wordpressVersion: string;
  passed: boolean;
  activePluginsTested: number;
  activeTheme?: string | null;
  output: string;
}
export interface CompatibilityMatrixReport {
  rows: CompatibilityMatrixRow[];
  passed: number;
  failed: number;
}
