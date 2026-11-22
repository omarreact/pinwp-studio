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
  tools: ToolProbe[];
}

export interface ProvisionResult {
  site: SiteRecord;
  siteUrl: string;
  adminUser: string;
  adminPassword: string;
}
