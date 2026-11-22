import type { CreateSiteInput, RuntimePreflight, SiteRecord } from '../types';

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function invokeNative<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    throw new Error('Native PinWP commands are available only inside the Tauri desktop application.');
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

export async function listSites(): Promise<SiteRecord[]> {
  if (!isTauri()) return [];
  return invokeNative<SiteRecord[]>('list_sites');
}

export async function createSite(input: CreateSiteInput): Promise<SiteRecord> {
  return invokeNative<SiteRecord>('create_site', { input });
}

export async function startSite(id: string): Promise<SiteRecord> {
  return invokeNative<SiteRecord>('start_site', { id });
}

export async function stopSite(id: string): Promise<SiteRecord> {
  return invokeNative<SiteRecord>('stop_site', { id });
}

export async function runtimePreflight(): Promise<RuntimePreflight> {
  if (!isTauri()) {
    return {
      appDataDir: 'Browser preview',
      runtimePackDir: 'Install the native desktop build to inspect runtime packs.',
      cachedWordpress: false,
      cachedSqliteIntegration: false,
      tools: [
        { name: 'PHP', available: false, source: 'Native app required' },
        { name: 'WP-CLI', available: false, source: 'Native app required' },
        { name: 'Git', available: false, source: 'Native app required' },
        { name: 'Cloudflared', available: false, source: 'Native app required' },
      ],
    };
  }
  return invokeNative<RuntimePreflight>('runtime_preflight');
}

export async function openExternal(target: string): Promise<void> {
  if (!isTauri()) {
    window.open(target, '_blank', 'noopener,noreferrer');
    return;
  }
  const { openUrl, openPath } = await import('@tauri-apps/plugin-opener');
  if (/^https?:\/\//i.test(target)) await openUrl(target);
  else await openPath(target);
}
