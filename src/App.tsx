import { FormEvent, useEffect, useMemo, useState } from 'react';
import {
  Activity, Boxes, ChevronRight, CircleStop, Cloud, Code2, Database, FolderOpen,
  Github, Globe2, HardDrive, Laptop, Moon, Play, Plus, RefreshCw, Search,
  Settings, ShieldCheck, Camera, SquareTerminal, Sun, Wrench,
} from 'lucide-react';
import { createSite, listSites, openExternal, provisionSite, runtimePreflight, startSite, stopSite } from './lib/tauri';
import type { CreateSiteInput, ProvisionResult, RuntimePreflight, SiteRecord } from './types';

const blankForm: CreateSiteInput = {
  name: '', slug: '', phpVersion: '8.3', wordpressVersion: 'Latest', databaseMode: 'sqlite',
};

function slugify(value: string) {
  return value.toLowerCase().trim().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 48);
}

function statusLabel(site?: SiteRecord) {
  if (!site) return 'No site selected';
  if (site.status === 'running') return 'Running';
  if (!site.provisioned) return 'Runtime setup required';
  return site.status === 'needsRuntime' ? 'Runtime missing' : 'Stopped';
}

export default function App() {
  const [sites, setSites] = useState<SiteRecord[]>([]);
  const [selectedId, setSelectedId] = useState<string>();
  const [query, setQuery] = useState('');
  const [tab, setTab] = useState('Overview');
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [preflight, setPreflight] = useState<RuntimePreflight>();
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string>();
  const [showCreate, setShowCreate] = useState(false);
  const [form, setForm] = useState<CreateSiteInput>(blankForm);
  const [credentials, setCredentials] = useState<ProvisionResult>();

  const refresh = async () => {
    const [nextSites, nextPreflight] = await Promise.all([listSites(), runtimePreflight()]);
    setSites(nextSites);
    setPreflight(nextPreflight);
    setSelectedId((current) => current && nextSites.some((site) => site.id === current) ? current : nextSites[0]?.id);
  };

  useEffect(() => { refresh().catch((error) => setNotice(error instanceof Error ? error.message : String(error))); }, []);
  useEffect(() => { document.documentElement.dataset.theme = theme; }, [theme]);

  const selected = sites.find((site) => site.id === selectedId);
  const filtered = sites.filter((site) => site.name.toLowerCase().includes(query.toLowerCase()));
  const toolSummary = useMemo(() => {
    if (!preflight) return 'Checking runtime…';
    return `${preflight.tools.filter((tool) => tool.available).length}/${preflight.tools.length} developer tools detected`;
  }, [preflight]);

  async function handleLifecycle(action: 'start' | 'stop') {
    if (!selected) return;
    setBusy(true);
    setNotice(undefined);
    try {
      const updated = action === 'start' ? await startSite(selected.id) : await stopSite(selected.id);
      setSites((current) => current.map((site) => site.id === updated.id ? updated : site));
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally { setBusy(false); }
  }

  async function handleProvision() {
    if (!selected) return;
    setBusy(true);
    setNotice(undefined);
    setCredentials(undefined);
    try {
      const result = await provisionSite(selected.id);
      setSites((current) => current.map((site) => site.id === result.site.id ? result.site : site));
      setCredentials(result);
      setNotice('WordPress installed completely from the local PinWP Runtime Pack. Save the one-time admin password shown below.');
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function handleCreate(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setNotice(undefined);
    try {
      const input = { ...form, slug: form.slug || slugify(form.name) };
      const site = await createSite(input);
      setSites((current) => [site, ...current]);
      setSelectedId(site.id);
      setShowCreate(false);
      setForm(blankForm);
      setTab('Overview');
      setNotice('Workspace created. Install a Runtime Pack or use detected PHP to start provisioning WordPress.');
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally { setBusy(false); }
  }

  const tabs = ['Overview', 'Database', 'Files', 'Logs', 'Git', 'Share', 'Deploy', 'Snapshots', 'Tools'];

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand"><div className="brand-mark">P</div><div><strong>PinWP</strong><span>Studio</span></div></div>
        <button className="new-site" onClick={() => setShowCreate(true)}><Plus size={16}/> New WordPress</button>
        <label className="search-box"><Search size={15}/><input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search sites"/></label>
        <div className="site-list">
          <div className="sidebar-label">Sites</div>
          {filtered.length === 0 && <div className="empty-sidebar">No local sites yet.</div>}
          {filtered.map((site) => (
            <button key={site.id} className={`site-row ${selectedId === site.id ? 'active' : ''}`} onClick={() => setSelectedId(site.id)}>
              <span className={`status-dot ${site.status === 'running' ? 'online' : ''}`}/>
              <span><strong>{site.name}</strong><small>{site.domain}</small></span><ChevronRight size={14}/>
            </button>
          ))}
        </div>
        <div className="sidebar-footer"><button><Boxes size={15}/> Blueprints</button><button><Settings size={15}/> Settings</button></div>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div><div className="eyebrow">LOCAL WORDPRESS WORKSTATION</div><h1>{selected?.name || 'PinWP Studio'}</h1></div>
          <div className="top-actions">
            <button className="icon-button" onClick={() => refresh()} title="Refresh"><RefreshCw size={17}/></button>
            <button className="icon-button" onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')} title="Toggle theme">{theme === 'dark' ? <Sun size={17}/> : <Moon size={17}/>}</button>
            {selected && !selected.provisioned && <button className="primary" disabled={busy} onClick={handleProvision}><Plus size={16}/> Install WordPress</button>}
            {selected && (selected.status === 'running'
              ? <button className="danger-soft" disabled={busy} onClick={() => handleLifecycle('stop')}><CircleStop size={16}/> Stop</button>
              : <button className="secondary" disabled={busy} onClick={() => handleLifecycle('start')}><Play size={16}/> Start</button>)}
          </div>
        </header>
        <nav className="tabbar">{tabs.map((item) => <button key={item} className={tab === item ? 'active' : ''} onClick={() => setTab(item)}>{item}</button>)}</nav>
        {notice && <div className="notice">{notice}</div>}

        {!selected ? (
          <section className="welcome"><div className="welcome-icon"><Laptop size={32}/></div><h2>Create your first local WordPress</h2><p>Each site gets its own workspace, local domain, logs, snapshots and deployment metadata.</p><button className="primary" onClick={() => setShowCreate(true)}><Plus size={16}/> Create WordPress</button></section>
        ) : (
          <div className="content">
            {tab === 'Overview' ? <>
              <section className="hero-grid">
                <div className="hero-card">
                  <div className="runtime-state"><span className={`pulse ${selected.status === 'running' ? 'online' : ''}`}/>{statusLabel(selected)}</div>
                  <h2>{selected.domain}</h2><p>Port {selected.port} · PHP {selected.phpVersion} · WordPress {selected.wordpressVersion}</p>
                  <div className="button-row">
                    <button className="primary" disabled={selected.status !== 'running'} onClick={() => openExternal(`http://127.0.0.1:${selected.port}`)}><Globe2 size={16}/> Open site</button>
                    <button className="secondary" disabled={!selected.provisioned} onClick={() => openExternal(`http://127.0.0.1:${selected.port}/wp-admin/`)}><ShieldCheck size={16}/> WP Admin</button>
                  </div>
                </div>
                <div className="environment-card">
                  <div className="section-title"><Wrench size={17}/> Environment</div>
                  <dl>
                    <div><dt>Runtime</dt><dd>Native PHP {selected.phpVersion}</dd></div>
                    <div><dt>Database</dt><dd>{selected.databaseMode === 'sqlite' ? 'SQLite / Lite' : 'MariaDB / Full'}</dd></div>
                    <div><dt>Workspace</dt><dd className="mono">{selected.path}</dd></div>
                    <div><dt>State</dt><dd>{selected.provisioned ? 'WordPress provisioned' : 'Workspace only'}</dd></div>
                  </dl>
                </div>
              </section>

              {credentials?.site.id === selected.id && (
                <section className="credentials-panel">
                  <div>
                    <span className="eyebrow">ONE-TIME LOCAL ADMIN</span>
                    <h3>WordPress is ready</h3>
                    <p>{credentials.siteUrl}</p>
                  </div>
                  <div className="credential-values">
                    <div><span>User</span><strong>{credentials.adminUser}</strong></div>
                    <div><span>Password</span><strong className="mono">{credentials.adminPassword}</strong></div>
                    <button className="secondary compact" onClick={() => navigator.clipboard.writeText(credentials.adminPassword)}>Copy password</button>
                  </div>
                </section>
              )}

              <section>
                <div className="section-heading"><div><span className="eyebrow">DEVELOPER SHORTCUTS</span><h3>Work without leaving PinWP</h3></div></div>
                <div className="quick-grid">
                  {[
                    [Code2, 'VS Code', 'Open project workspace'], [SquareTerminal, 'WP-CLI', 'Run controlled commands'],
                    [Database, 'Database', 'Inspect local data'], [FolderOpen, 'Files', 'Reveal workspace'],
                    [Cloud, 'Share', 'Tunnel or LAN preview'], [Camera, 'Snapshot', 'Create restore point'],
                  ].map(([Icon, title, copy]) => {
                    const Comp = Icon as typeof Code2;
                    return <button className="quick-card" key={String(title)} onClick={() => title === 'Files' && openExternal(selected.path)}><Comp size={19}/><strong>{String(title)}</strong><span>{String(copy)}</span></button>;
                  })}
                </div>
              </section>

              <section className="runtime-panel">
                <div className="section-heading"><div><span className="eyebrow">RUNTIME PREFLIGHT</span><h3>{toolSummary}</h3></div><button className="secondary compact" onClick={() => refresh()}><RefreshCw size={14}/> Recheck</button></div>
                <div className="tool-grid">{preflight?.tools.map((tool) => <div className="tool-card" key={tool.name}><span className={`tool-indicator ${tool.available ? 'ok' : ''}`}/><div><strong>{tool.name}</strong><small>{tool.version || tool.source || 'Not detected'}</small></div></div>)}</div>
                <div className="runtime-note"><HardDrive size={16}/><span>Runtime Packs will bundle/cache PHP, WordPress core, SQLite integration, MariaDB and cloudflared so the installed app can work fully offline after setup.</span></div>
              </section>
            </> : (
              <section className="placeholder-panel"><div className="placeholder-icon">{tab === 'Database' ? <Database/> : tab === 'Git' ? <Github/> : tab === 'Share' ? <Cloud/> : <Activity/>}</div><span className="eyebrow">MVP MODULE</span><h2>{tab}</h2><p>The navigation and data model are in place. This module is scheduled after the native Runtime Pack lifecycle is stabilized.</p></section>
            )}
          </div>
        )}
      </main>

      {showCreate && (
        <div className="modal-backdrop" onMouseDown={() => setShowCreate(false)}>
          <form className="modal" onSubmit={handleCreate} onMouseDown={(e) => e.stopPropagation()}>
            <div className="modal-head"><div><span className="eyebrow">NEW ENVIRONMENT</span><h2>Create WordPress workspace</h2></div><button type="button" className="icon-button" onClick={() => setShowCreate(false)}>×</button></div>
            <label>Site name<input autoFocus required value={form.name} onChange={(e) => setForm({...form,name:e.target.value,slug:slugify(e.target.value)})} placeholder="Meta Grey Label"/></label>
            <label>Local slug<div className="domain-input"><input required value={form.slug} onChange={(e) => setForm({...form,slug:slugify(e.target.value)})} placeholder="meta-grey-label"/><span>.wpdev.local</span></div></label>
            <div className="choice-grid">
              <button type="button" className={form.databaseMode === 'sqlite' ? 'choice active' : 'choice'} onClick={() => setForm({...form,databaseMode:'sqlite'})}><Database size={20}/><strong>Lite</strong><span>Native PHP + SQLite integration</span></button>
              <button type="button" className={form.databaseMode === 'mariadb' ? 'choice active' : 'choice'} onClick={() => setForm({...form,databaseMode:'mariadb'})}><Database size={20}/><strong>Full</strong><span>Native PHP + MariaDB compatibility</span></button>
            </div>
            <div className="form-grid">
              <label>PHP<select value={form.phpVersion} onChange={(e) => setForm({...form,phpVersion:e.target.value})}><option>8.4</option><option>8.3</option><option>8.2</option><option>8.1</option></select></label>
              <label>WordPress<select value={form.wordpressVersion} onChange={(e) => setForm({...form,wordpressVersion:e.target.value})}><option>Latest</option><option>7.1</option><option>7.0</option><option>6.9</option></select></label>
            </div>
            <div className="modal-actions"><button type="button" className="secondary" onClick={() => setShowCreate(false)}>Cancel</button><button className="primary" disabled={busy || !form.name || !form.slug}><Plus size={16}/> Create workspace</button></div>
          </form>
        </div>
      )}
    </div>
  );
}
