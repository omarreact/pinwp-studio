import type {
  CompatibilityReport, CreateSiteInput, DatabaseResult, DeployInput, DeployResult,
  GitInfo, LogRecord, ProvisionResult, RouterInfo, RuntimePreflight, ShareInfo,
  SiteRecord, SnapshotRecord,
} from '../types';

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function invokeNative<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) throw new Error('Native PinWP commands are available only inside the Tauri desktop application.');
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

export const listSites = async (): Promise<SiteRecord[]> => isTauri() ? invokeNative('list_sites') : [];
export const createSite = (input: CreateSiteInput) => invokeNative<SiteRecord>('create_site', { input });
export const startSite = (id: string) => invokeNative<SiteRecord>('start_site', { id });
export const stopSite = (id: string) => invokeNative<SiteRecord>('stop_site', { id });
export const provisionSite = (id: string) => invokeNative<ProvisionResult>('provision_site', { id });
export const databaseQuery = (id: string, sql: string, allowWrite: boolean) => invokeNative<DatabaseResult>('database_query', { id, sql, allowWrite });
export const listSnapshots = (id: string) => invokeNative<SnapshotRecord[]>('list_snapshots', { id });
export const createSnapshot = (id: string, name: string) => invokeNative<SnapshotRecord>('create_snapshot', { id, name });
export const restoreSnapshot = (id: string, snapshotId: string) => invokeNative<void>('restore_snapshot', { id, snapshotId });
export const deleteSnapshot = (id: string, snapshotId: string) => invokeNative<void>('delete_snapshot', { id, snapshotId });
export const exportSite = (id: string) => invokeNative<string>('export_site', { id });
export const importSite = (id: string, archive: string) => invokeNative<void>('import_site', { id, archive });
export const listLogs = (id: string) => invokeNative<LogRecord[]>('list_logs', { id });
export const readLog = (path: string, maxBytes = 200_000) => invokeNative<string>('read_log', { path, maxBytes });
export const gitStatus = (id: string) => invokeNative<GitInfo>('git_status', { id });
export const gitInit = (id: string) => invokeNative<GitInfo>('git_init', { id });
export const gitCommit = (id: string, message: string) => invokeNative<GitInfo>('git_commit', { id, message });
export const openCode = (id: string) => invokeNative<void>('open_code', { id });
export const startTunnel = (id: string) => invokeNative<ShareInfo>('start_tunnel', { id });
export const stopTunnel = (id: string) => invokeNative<void>('stop_tunnel', { id });
export const startMailpit = () => invokeNative<string>('start_mailpit');
export const stopMailpit = () => invokeNative<void>('stop_mailpit');
export const routerStatus = (id: string) => invokeNative<RouterInfo>('router_status', { id });
export const enableRouter = (id: string) => invokeNative<RouterInfo>('enable_router', { id });
export const disableRouter = (id: string) => invokeNative<RouterInfo>('disable_router', { id });
export const runWpCli = (id: string, args: string[], allowUnsafe: boolean) => invokeNative<string>('run_wp_cli', { id, args, allowUnsafe });
export const compatibilityReport = (id: string) => invokeNative<CompatibilityReport>('compatibility_report', { id });
export const testDeployment = (input: DeployInput) => invokeNative<DeployResult>('test_deployment', { input });
export const deploySite = (input: DeployInput) => invokeNative<DeployResult>('deploy_site', { input });

export async function runtimePreflight(): Promise<RuntimePreflight> {
  if (!isTauri()) return {
    appDataDir: 'Browser preview', runtimePackDir: 'Native app required',
    cachedWordpress: false, cachedSqliteIntegration: false, cachedMariadb: false,
    cachedCloudflared: false, cachedCaddy: false, cachedMailpit: false,
    tools: [],
  };
  return invokeNative<RuntimePreflight>('runtime_preflight');
}

export async function openExternal(target: string): Promise<void> {
  if (!isTauri()) { window.open(target, '_blank', 'noopener,noreferrer'); return; }
  const { openUrl, openPath } = await import('@tauri-apps/plugin-opener');
  if (/^https?:\/\//i.test(target)) await openUrl(target); else await openPath(target);
}
