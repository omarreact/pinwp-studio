import { FormEvent, useEffect, useMemo, useState } from 'react';
import {
  Activity, Boxes, ChevronRight, CircleStop, Cloud, Code2, Database, Download,
  FileArchive, FileText, FolderOpen, Github, Globe2, HardDrive, Laptop, Mail,
  Moon, Play, Plus, RefreshCw, RotateCcw, Search, Settings, ShieldCheck, Camera,
  SquareTerminal, Sun, Trash2, Upload, Wrench,
} from 'lucide-react';
import * as api from './lib/tauri';
import type {
  CompatibilityMatrixReport, CompatibilityReport, CreateSiteInput, DeployInput, GitInfo, LogRecord,
  ProvisionResult, RouterInfo, RuntimePreflight, RuntimeVersionCatalog, ShareInfo, SiteRecord, SnapshotRecord,
} from './types';

const blankForm: CreateSiteInput = { name: '', slug: '', phpVersion: '8.4', wordpressVersion: 'Latest', databaseMode: 'sqlite' };
const defaultDeploy: DeployInput = { siteId: '', kind: 'ssh', host: '', port: 22, username: '', secret: '', remotePath: 'public_html', siteUrl: '' };

function slugify(value: string) {
  return value.toLowerCase().trim().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 48);
}
function statusLabel(site?: SiteRecord) {
  if (!site) return 'No site selected';
  if (site.status === 'running') return 'Running';
  if (!site.provisioned) return 'WordPress not installed';
  return site.status === 'error' ? 'Needs attention' : 'Stopped';
}
function splitArgs(value: string) {
  const re = /"([^"]*)"|'([^']*)'|(\S+)/g; const out: string[] = []; let match: RegExpExecArray | null;
  while ((match = re.exec(value))) out.push(match[1] ?? match[2] ?? match[3]);
  return out;
}
function bytes(value: number) {
  if (value < 1024) return value + ' B';
  if (value < 1024 * 1024) return (value / 1024).toFixed(1) + ' KB';
  return (value / 1024 / 1024).toFixed(1) + ' MB';
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
  const [dbSql, setDbSql] = useState('SELECT option_name, option_value FROM wp_options LIMIT 25;');
  const [dbWrite, setDbWrite] = useState(false);
  const [dbOutput, setDbOutput] = useState('');
  const [snapshots, setSnapshots] = useState<SnapshotRecord[]>([]);
  const [snapshotName, setSnapshotName] = useState('');
  const [logs, setLogs] = useState<LogRecord[]>([]);
  const [logOutput, setLogOutput] = useState('');
  const [git, setGit] = useState<GitInfo>();
  const [commitMessage, setCommitMessage] = useState('');
  const [share, setShare] = useState<ShareInfo>();
  const [namedTunnelToken, setNamedTunnelToken] = useState('');
  const [namedTunnelHostname, setNamedTunnelHostname] = useState('');
  const [router, setRouter] = useState<RouterInfo>();
  const [mailpitUrl, setMailpitUrl] = useState<string>();
  const [archivePath, setArchivePath] = useState('');
  const [wpCli, setWpCli] = useState('plugin list');
  const [wpCliOutput, setWpCliOutput] = useState('');
  const [unsafeCli, setUnsafeCli] = useState(false);
  const [compatibility, setCompatibility] = useState<CompatibilityReport>();
  const [matrixReport, setMatrixReport] = useState<CompatibilityMatrixReport>();
  const [deploy, setDeploy] = useState<DeployInput>(defaultDeploy);
  const [deployOutput, setDeployOutput] = useState('');
  const [runtimeVersions, setRuntimeVersions] = useState<RuntimeVersionCatalog>();

  const refresh = async () => {
    const [nextSites, nextPreflight] = await Promise.all([api.listSites(), api.runtimePreflight()]);
    setSites(nextSites); setPreflight(nextPreflight);
    setSelectedId((current) => current && nextSites.some((site) => site.id === current) ? current : nextSites[0]?.id);
  };
  useEffect(() => { refresh().catch((e) => setNotice(String(e))); }, []);
  useEffect(() => { document.documentElement.dataset.theme = theme; }, [theme]);

  const selected = sites.find((site) => site.id === selectedId);
  const filtered = sites.filter((site) => site.name.toLowerCase().includes(query.toLowerCase()));
  const toolSummary = useMemo(() => preflight ? `${preflight.tools.filter((t) => t.available).length}/${preflight.tools.length} host tools detected` : 'Checking runtime…', [preflight]);

  useEffect(() => {
    if (!selected) return;
    setDeploy((d) => ({ ...d, siteId: selected.id }));
    if (tab === 'Snapshots') api.listSnapshots(selected.id).then(setSnapshots).catch(() => {});
    if (tab === 'Logs') api.listLogs(selected.id).then(setLogs).catch(() => {});
    if (tab === 'Git') api.gitStatus(selected.id).then(setGit).catch(() => {});
    if (tab === 'Share') api.routerStatus(selected.id).then(setRouter).catch(() => {});
    if (tab === 'Tools') api.runtimeVersions().then(setRuntimeVersions).catch(() => {});
  }, [selected?.id, tab]);

  async function run<T>(fn: () => Promise<T>, success?: string): Promise<T | undefined> {
    setBusy(true); setNotice(undefined);
    try { const value = await fn(); if (success) setNotice(success); return value; }
    catch (e) { setNotice(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  }

  async function handleLifecycle(action: 'start' | 'stop') {
    if (!selected) return;
    const updated = await run(() => action === 'start' ? api.startSite(selected.id) : api.stopSite(selected.id));
    if (updated) setSites((current) => current.map((s) => s.id === updated.id ? updated : s));
  }
  async function handleProvision() {
    if (!selected) return;
    setCredentials(undefined);
    const result = await run(() => api.provisionSite(selected.id), 'WordPress installed from the local Runtime Pack.');
    if (result) { setSites((current) => current.map((s) => s.id === result.site.id ? result.site : s)); setCredentials(result); }
  }
  async function handleCreate(event: FormEvent) {
    event.preventDefault();
    const site = await run(() => api.createSite({ ...form, slug: form.slug || slugify(form.name) }), 'Workspace created.');
    if (site) { setSites((c) => [site, ...c]); setSelectedId(site.id); setShowCreate(false); setForm(blankForm); setTab('Overview'); }
  }

  const tabs = ['Overview', 'Database', 'Files', 'Logs', 'Git', 'Share', 'Deploy', 'Snapshots', 'Tools'];
  const localUrl = selected ? `http://${selected.domain}:${selected.port}` : '';

  return <div className="app-shell">
    <aside className="sidebar">
      <div className="brand"><div className="brand-mark">P</div><div><strong>PinWP</strong><span>Studio v1</span></div></div>
      <button className="new-site" onClick={() => setShowCreate(true)}><Plus size={16}/> New WordPress</button>
      <label className="search-box"><Search size={15}/><input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search sites"/></label>
      <div className="site-list"><div className="sidebar-label">Sites</div>{filtered.length === 0 && <div className="empty-sidebar">No local sites yet.</div>}
        {filtered.map((site) => <button key={site.id} className={`site-row ${selectedId === site.id ? 'active' : ''}`} onClick={() => setSelectedId(site.id)}>
          <span className={`status-dot ${site.status === 'running' ? 'online' : ''}`}/><span><strong>{site.name}</strong><small>{site.domain}:{site.port}</small></span><ChevronRight size={14}/>
        </button>)}
      </div>
      <div className="sidebar-footer"><button onClick={() => setTab('Snapshots')}><Boxes size={15}/> Snapshots</button><button onClick={() => setTab('Tools')}><Settings size={15}/> Tools</button></div>
    </aside>

    <main className="workspace">
      <header className="topbar"><div><div className="eyebrow">OFFLINE-FIRST WORDPRESS WORKSTATION</div><h1>{selected?.name || 'PinWP Studio'}</h1></div>
        <div className="top-actions">
          <button className="icon-button" onClick={() => refresh()}><RefreshCw size={17}/></button>
          <button className="icon-button" onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}>{theme === 'dark' ? <Sun size={17}/> : <Moon size={17}/>}</button>
          {selected && !selected.provisioned && <button className="primary" disabled={busy} onClick={handleProvision}><Plus size={16}/> Install WordPress</button>}
          {selected && (selected.status === 'running' ? <button className="danger-soft" disabled={busy} onClick={() => handleLifecycle('stop')}><CircleStop size={16}/> Stop</button> : <button className="secondary" disabled={busy || !selected.provisioned} onClick={() => handleLifecycle('start')}><Play size={16}/> Start</button>)}
        </div>
      </header>
      <nav className="tabbar">{tabs.map((item) => <button key={item} className={tab === item ? 'active' : ''} onClick={() => setTab(item)}>{item}</button>)}</nav>
      {notice && <div className="notice">{notice}</div>}

      {!selected ? <section className="welcome"><div className="welcome-icon"><Laptop size={32}/></div><h2>Create your first local WordPress</h2><p>Lite mode uses SQLite. Full mode uses the bundled MariaDB runtime.</p><button className="primary" onClick={() => setShowCreate(true)}><Plus size={16}/> Create WordPress</button></section> :
      <div className="content">
        {tab === 'Overview' && <>
          <section className="hero-grid">
            <div className="hero-card"><div className="runtime-state"><span className={`pulse ${selected.status === 'running' ? 'online' : ''}`}/>{statusLabel(selected)}</div>
              <h2>{selected.domain}:{selected.port}</h2><p>PHP {selected.phpVersion} · {selected.databaseMode === 'sqlite' ? 'SQLite Lite' : 'MariaDB Full'} · WordPress {selected.wordpressVersion}</p>
              <div className="button-row"><button className="primary" disabled={selected.status !== 'running'} onClick={() => api.openExternal(localUrl)}><Globe2 size={16}/> Open site</button><button className="secondary" disabled={selected.status !== 'running'} onClick={() => api.openExternal(localUrl + '/wp-admin/')}><ShieldCheck size={16}/> WP Admin</button></div>
            </div>
            <div className="environment-card"><div className="section-title"><Wrench size={17}/> Environment</div><dl>
              <div><dt>Database</dt><dd>{selected.databaseMode}</dd></div><div><dt>HTTPS</dt><dd>{selected.httpsEnabled ? selected.customDomain : 'Optional'}</dd></div><div><dt>Workspace</dt><dd className="mono">{selected.path}</dd></div><div><dt>Snapshot</dt><dd>{selected.lastSnapshotAt || 'None'}</dd></div>
            </dl></div>
          </section>
          {credentials?.site.id === selected.id && <section className="credentials-panel"><div><span className="eyebrow">ONE-TIME LOCAL ADMIN</span><h3>WordPress is ready</h3><p>{credentials.siteUrl}</p></div><div className="credential-values"><div><span>User</span><strong>{credentials.adminUser}</strong></div><div><span>Password</span><strong className="mono">{credentials.adminPassword}</strong></div><button className="secondary compact" onClick={() => navigator.clipboard.writeText(credentials.adminPassword)}>Copy</button></div></section>}
          <section><div className="section-heading"><div><span className="eyebrow">WORKSPACE</span><h3>Developer shortcuts</h3></div></div><div className="quick-grid">
            {[[Code2,'VS Code','Open project'],[SquareTerminal,'WP-CLI','WordPress CLI'],[Database,'Database','Query local DB'],[FolderOpen,'Files','Open workspace'],[Cloud,'Share','Tunnel / HTTPS'],[Camera,'Snapshot','Restore point']].map(([Icon,title,copy]) => { const Comp=Icon as typeof Code2; return <button className="quick-card" key={String(title)} onClick={() => title === 'Files' ? api.openExternal(selected.path) : title === 'VS Code' ? run(() => api.openCode(selected.id)) : setTab(title === 'WP-CLI' ? 'Tools' : String(title) === 'Snapshot' ? 'Snapshots' : String(title))}><Comp size={19}/><strong>{String(title)}</strong><span>{String(copy)}</span></button>; })}
          </div></section>
          <section className="runtime-panel"><div className="section-heading"><div><span className="eyebrow">RUNTIME V1</span><h3>{toolSummary}</h3></div></div>
            <div className="tool-grid">{[['WordPress',preflight?.cachedWordpress],['SQLite',preflight?.cachedSqliteIntegration],['MariaDB',preflight?.cachedMariadb],['Cloudflare',preflight?.cachedCloudflared],['Caddy',preflight?.cachedCaddy],['Mailpit',preflight?.cachedMailpit]].map(([name,ok]) => <div className="tool-card" key={String(name)}><span className={`tool-indicator ${ok ? 'ok' : ''}`}/><div><strong>{String(name)}</strong><small>{ok ? 'Bundled' : 'Missing'}</small></div></div>)}</div>
          </section>
        </>}

        {tab === 'Database' && <section className="module-panel"><div className="section-heading"><div><span className="eyebrow">DATABASE</span><h3>{selected.databaseMode === 'sqlite' ? 'SQLite browser' : 'MariaDB browser'}</h3></div></div>
          <textarea className="code-area" value={dbSql} onChange={(e) => setDbSql(e.target.value)}/><label className="check-row"><input type="checkbox" checked={dbWrite} onChange={(e) => setDbWrite(e.target.checked)}/> Allow write SQL for this run</label>
          <div className="button-row"><button className="primary" disabled={busy} onClick={async()=>{const r=await run(()=>api.databaseQuery(selected.id,dbSql,dbWrite));if(r)setDbOutput(r.output)}}><Play size={15}/> Run query</button><button className="secondary" onClick={()=>setDbSql(selected.databaseMode==='sqlite'?"SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;":"SHOW TABLES;")}>List tables</button></div>
          <pre className="output">{dbOutput || 'Query output will appear here.'}</pre>
        </section>}

        {tab === 'Files' && <section className="module-panel"><div className="section-heading"><div><span className="eyebrow">FILES & MIGRATION</span><h3>Import / export workspace</h3></div></div>
          <div className="action-grid"><button className="action-card" onClick={()=>api.openExternal(selected.path)}><FolderOpen/><strong>Open workspace</strong><span>{selected.path}</span></button><button className="action-card" onClick={async()=>{const p=await run(()=>api.exportSite(selected.id),'Export package created.');if(p)setArchivePath(p)}}><Download/><strong>Export .pinwp.zip</strong><span>Files + database + manifest</span></button><button className="action-card" onClick={async()=>{const p=await run(()=>api.exportDatabaseSql(selected.id),'MySQL-compatible SQL exported.');if(p)setArchivePath(p)}}><Database/><strong>Export MySQL SQL</strong><span>MariaDB dump or SQLite → MySQL conversion</span></button></div>
          <label className="field">Archive path<input value={archivePath} onChange={(e)=>setArchivePath(e.target.value)} placeholder="C:\path\site.pinwp.zip"/></label>
          <button className="secondary" disabled={!archivePath||busy} onClick={async()=>{if(!window.confirm('Import will replace the current workspace files and database. Continue?'))return;await run(()=>api.importSite(selected.id,archivePath),'Import completed.');await refresh();}}><Upload size={15}/> Import into this workspace</button>
        </section>}

        {tab === 'Logs' && <section className="module-panel"><div className="section-heading"><div><span className="eyebrow">LOGS</span><h3>Runtime and WordPress logs</h3></div><button className="secondary compact" onClick={async()=>setLogs(await api.listLogs(selected.id))}><RefreshCw size={14}/> Refresh</button></div>
          <div className="log-list">{logs.map((log)=><button key={log.path} onClick={async()=>setLogOutput(await api.readLog(log.path))}><FileText size={15}/><span><strong>{log.name}</strong><small>{bytes(log.sizeBytes)}</small></span></button>)}</div><pre className="output tall">{logOutput || 'Select a log file.'}</pre>
        </section>}

        {tab === 'Git' && <section className="module-panel"><div className="section-heading"><div><span className="eyebrow">GIT</span><h3>{git?.initialized ? git.branch || 'Repository' : 'Version control'}</h3></div><button className="secondary compact" onClick={async()=>setGit(await api.gitStatus(selected.id))}><RefreshCw size={14}/> Refresh</button></div>
          <pre className="output">{git?.output || 'Checking Git…'}</pre>
          {!git?.initialized ? <button className="primary" onClick={async()=>{const g=await run(()=>api.gitInit(selected.id));if(g)setGit(g)}}><Github size={15}/> Initialize Git</button> : <div className="inline-form"><input value={commitMessage} onChange={(e)=>setCommitMessage(e.target.value)} placeholder="Commit message"/><button className="primary" onClick={async()=>{const g=await run(()=>api.gitCommit(selected.id,commitMessage));if(g){setGit(g);setCommitMessage('')}}}>Commit all</button></div>}
        </section>}

        {tab === 'Share' && <section className="module-panel"><div className="section-heading"><div><span className="eyebrow">SHARE & ROUTING</span><h3>Local HTTPS, tunnel and mail</h3></div></div>
          <div className="action-grid three">
            <div className="service-card"><Cloud/><strong>Cloudflare Quick Tunnel</strong><span>{share?.url || 'Ephemeral public preview'}</span><div className="button-row"><button className="primary" disabled={selected.status!=='running'} onClick={async()=>{const s=await run(()=>api.startTunnel(selected.id));if(s)setShare(s)}}>Start</button><button className="secondary" onClick={()=>run(()=>api.stopTunnel(selected.id),'Tunnel stopped.')}>Stop</button></div></div><div className="service-card"><Cloud/><strong>Persistent Tunnel</strong><span>Use a Cloudflare dashboard tunnel token; Access protection can be applied to its hostname.</span><input className="service-input" type="password" value={namedTunnelToken} onChange={(e)=>setNamedTunnelToken(e.target.value)} placeholder="Tunnel token"/><input className="service-input" value={namedTunnelHostname} onChange={(e)=>setNamedTunnelHostname(e.target.value)} placeholder="dev.example.com"/><div className="button-row"><button className="primary" disabled={selected.status!=='running'||!namedTunnelToken} onClick={async()=>{const s=await run(()=>api.startNamedTunnel(selected.id,namedTunnelToken,namedTunnelHostname),'Persistent tunnel started. Token was not saved.');if(s)setShare(s)}}>Start</button><button className="secondary" onClick={()=>run(()=>api.stopTunnel(selected.id),'Tunnel stopped.')}>Stop</button></div></div>
            <div className="service-card"><ShieldCheck/><strong>Clean local HTTPS</strong><span>{router?.url || `${selected.slug}.wpdev.local`}</span><div className="button-row">{router?.enabled?<button className="secondary" onClick={async()=>{const r=await run(()=>api.disableRouter(selected.id));if(r)setRouter(r)}}>Disable</button>:<button className="primary" onClick={async()=>{const r=await run(()=>api.enableRouter(selected.id),'Windows will request administrator permission to update hosts/trust the local CA.');if(r)setRouter(r)}}>Enable</button>}</div></div>
            <div className="service-card"><Mail/><strong>Mailpit</strong><span>{mailpitUrl || 'SMTP 1025 · UI 8025'}</span><div className="button-row"><button className="primary" onClick={async()=>{const url=await run(()=>api.startMailpit());if(url){setMailpitUrl(url);api.openExternal(url)}}}>Start / Open</button><button className="secondary" onClick={()=>run(()=>api.stopMailpit(),'Mailpit stopped.')}>Stop</button></div></div>
          </div>
          {share?.url && <button className="link-box" onClick={()=>api.openExternal(share.url!)}>{share.url}</button>}
        </section>}

        {tab === 'Deploy' && <section className="module-panel"><div className="section-heading"><div><span className="eyebrow">DEPLOY</span><h3>SSH / cPanel / PinWP Connector</h3></div></div>
          <div className="form-grid deploy-form">
            <label>Adapter<select value={deploy.kind} onChange={(e)=>{const kind=e.target.value as DeployInput['kind'];setDeploy({...deploy,kind,port:kind==='ssh'?22:kind==='cpanel'?2083:null})}}><option value="ssh">SSH/SCP</option><option value="cpanel">cPanel UAPI</option><option value="connector">PinWP Connector</option></select></label>
            <label>Host<input value={deploy.host} onChange={(e)=>setDeploy({...deploy,host:e.target.value})} placeholder="example.com"/></label>
            <label>Port<input type="number" value={deploy.port ?? ''} onChange={(e)=>setDeploy({...deploy,port:e.target.value?Number(e.target.value):null})}/></label>
            <label>Username<input value={deploy.username} onChange={(e)=>setDeploy({...deploy,username:e.target.value})}/></label>
            <label>Secret / token / app password<input type="password" value={deploy.secret} onChange={(e)=>setDeploy({...deploy,secret:e.target.value})}/></label>
            <label>Remote path<input value={deploy.remotePath} onChange={(e)=>setDeploy({...deploy,remotePath:e.target.value})}/></label>
            <label className="span-2">Remote site URL<input value={deploy.siteUrl} onChange={(e)=>setDeploy({...deploy,siteUrl:e.target.value})} placeholder="https://example.com"/></label>
          </div>
          <p className="module-note">Secrets are used for the current action only and are not written to the site registry. SSH uses your Windows OpenSSH key/agent; the secret field is ignored for SSH.</p>
          <div className="button-row"><button className="secondary" onClick={async()=>{const r=await run(()=>api.testDeployment({...deploy,siteId:selected.id}));if(r)setDeployOutput(r.message)}}>Test connection</button><button className="primary" onClick={async()=>{const r=await run(()=>api.deploySite({...deploy,siteId:selected.id}));if(r)setDeployOutput(r.message)}}>Backup + Deploy</button><button className="danger-soft" disabled={deploy.kind==='cpanel'} onClick={async()=>{if(!window.confirm('Restore the latest PinWP remote backup?'))return;const r=await run(()=>api.rollbackDeployment({...deploy,siteId:selected.id}));if(r)setDeployOutput(r.message)}}><RotateCcw size={15}/> Rollback latest</button></div>
          <pre className="output">{deployOutput || 'Deployment output will appear here.'}</pre>
        </section>}

        {tab === 'Snapshots' && <section className="module-panel"><div className="section-heading"><div><span className="eyebrow">SNAPSHOTS</span><h3>Local restore points</h3></div><button className="secondary compact" onClick={async()=>setSnapshots(await api.listSnapshots(selected.id))}><RefreshCw size={14}/> Refresh</button></div>
          <div className="inline-form"><input value={snapshotName} onChange={(e)=>setSnapshotName(e.target.value)} placeholder="Before theme update"/><button className="primary" onClick={async()=>{const s=await run(()=>api.createSnapshot(selected.id,snapshotName),'Snapshot created.');if(s){setSnapshots([s,...snapshots]);setSnapshotName('');await refresh();}}}><Camera size={15}/> Create snapshot</button></div>
          <div className="snapshot-list">{snapshots.map((s)=><div key={s.id}><div><strong>{s.name}</strong><span>{new Date(s.createdAt).toLocaleString()} · {bytes(s.sizeBytes)} · {s.databaseMode}</span></div><div className="button-row"><button className="secondary compact" onClick={async()=>{if(!window.confirm('Restore this snapshot? Current files/database will be replaced.'))return;await run(()=>api.restoreSnapshot(selected.id,s.id),'Snapshot restored.');await refresh();}}><RotateCcw size={13}/> Restore</button><button className="danger-soft compact" onClick={async()=>{if(!window.confirm('Delete this snapshot permanently?'))return;await run(()=>api.deleteSnapshot(selected.id,s.id),'Snapshot deleted.');setSnapshots(snapshots.filter(x=>x.id!==s.id))}}><Trash2 size={13}/></button></div></div>)}</div>
        </section>}

        {tab === 'Tools' && <section className="module-panel"><div className="section-heading"><div><span className="eyebrow">TOOLS & QA</span><h3>Runtime versions, WP-CLI and compatibility</h3></div></div>
          <div className="version-manager">
            <div><h4>PHP Versions</h4><div className="version-list">{runtimeVersions?.php.map((v)=><div key={v.key}><span><strong>PHP {v.key}</strong><small>{v.version}{v.bundled?' · bundled':''}</small></span>{v.installed?<button className="secondary compact" disabled={v.bundled||busy} onClick={async()=>{const c=await run(()=>api.removeRuntimeVersion('php',v.key),'PHP version removed.');if(c)setRuntimeVersions(c)}}>{v.bundled?'Default':'Remove'}</button>:<button className="primary compact" disabled={busy} onClick={async()=>{const c=await run(()=>api.installRuntimeVersion('php',v.key),'PHP version installed and cached.');if(c)setRuntimeVersions(c)}}>Install</button>}</div>)}</div></div>
            <div><h4>WordPress Versions</h4><div className="version-list">{runtimeVersions?.wordpress.map((v)=><div key={v.key}><span><strong>WordPress {v.key}</strong><small>{v.version}{v.bundled?' · bundled':''}</small></span>{v.installed?<button className="secondary compact" disabled={v.bundled||busy} onClick={async()=>{const c=await run(()=>api.removeRuntimeVersion('wordpress',v.key),'WordPress version removed.');if(c)setRuntimeVersions(c)}}>{v.bundled?'Default':'Remove'}</button>:<button className="primary compact" disabled={busy} onClick={async()=>{const c=await run(()=>api.installRuntimeVersion('wordpress',v.key),'WordPress version installed and cached.');if(c)setRuntimeVersions(c)}}>Install</button>}</div>)}</div></div>
          </div>
          <label className="field">WP-CLI arguments<input value={wpCli} onChange={(e)=>setWpCli(e.target.value)} placeholder="plugin list"/></label><label className="check-row"><input type="checkbox" checked={unsafeCli} onChange={(e)=>setUnsafeCli(e.target.checked)}/> Allow eval / shell commands</label>
          <div className="button-row"><button className="primary" onClick={async()=>{const out=await run(()=>api.runWpCli(selected.id,splitArgs(wpCli),unsafeCli));if(out!==undefined)setWpCliOutput(out)}}><SquareTerminal size={15}/> Run WP-CLI</button><button className="secondary" onClick={()=>run(()=>api.openCode(selected.id),'VS Code opened.')}><Code2 size={15}/> Open VS Code</button><button className="secondary" onClick={async()=>{const c=await run(()=>api.compatibilityReport(selected.id));if(c)setCompatibility(c)}}><Activity size={15}/> Compatibility check</button><button className="secondary" onClick={async()=>{const m=await run(()=>api.compatibilityMatrix(selected.id),'Compatibility matrix completed.');if(m)setMatrixReport(m)}}><Activity size={15}/> Run full matrix</button></div>
          <pre className="output">{wpCliOutput || compatibility?.output || 'Tool output will appear here.'}</pre>{matrixReport&&<div className="matrix-wrap"><div className="matrix-summary"><strong>{matrixReport.passed} passed</strong><span>{matrixReport.failed} failed</span></div><div className="matrix-table"><div className="matrix-head"><span>PHP</span><span>WordPress</span><span>Result</span><span>Project smoke</span></div>{matrixReport.rows.map((row)=><div key={row.phpVersion+'-'+row.wordpressVersion} className={row.passed?'matrix-row pass':'matrix-row fail'}><span>{row.phpVersion}</span><span>{row.wordpressVersion}</span><strong>{row.passed?'PASS':'FAIL'}</strong><span title={row.output}>{row.activePluginsTested} plugins{row.activeTheme?' · '+row.activeTheme:''}</span></div>)}</div></div>}
        </section>}
      </div>}

      {showCreate && <div className="modal-backdrop" onMouseDown={()=>setShowCreate(false)}><form className="modal" onSubmit={handleCreate} onMouseDown={(e)=>e.stopPropagation()}>
        <div className="modal-head"><div><span className="eyebrow">NEW ENVIRONMENT</span><h2>Create WordPress workspace</h2></div><button type="button" className="icon-button" onClick={()=>setShowCreate(false)}>×</button></div>
        <label>Site name<input autoFocus required value={form.name} onChange={(e)=>setForm({...form,name:e.target.value,slug:slugify(e.target.value)})} placeholder="Meta Grey Label"/></label>
        <label>Local slug<div className="domain-input"><input required value={form.slug} onChange={(e)=>setForm({...form,slug:slugify(e.target.value)})}/><span>.localhost</span></div></label>
        <div className="choice-grid"><button type="button" className={form.databaseMode==='sqlite'?'choice active':'choice'} onClick={()=>setForm({...form,databaseMode:'sqlite'})}><Database size={20}/><strong>Lite</strong><span>PHP + SQLite · zero DB server</span></button><button type="button" className={form.databaseMode==='mariadb'?'choice active':'choice'} onClick={()=>setForm({...form,databaseMode:'mariadb'})}><Database size={20}/><strong>Full</strong><span>PHP + bundled MariaDB</span></button></div>
        <div className="form-grid"><label>PHP<select value={form.phpVersion} onChange={(e)=>setForm({...form,phpVersion:e.target.value})}><option>8.5</option><option>8.4</option><option>8.3</option><option>8.2</option><option>8.1</option></select></label><label>WordPress<select value={form.wordpressVersion} onChange={(e)=>setForm({...form,wordpressVersion:e.target.value})}><option>Latest</option><option>7.1</option><option>7.0</option><option>6.9</option></select></label></div>
        <div className="modal-actions"><button type="button" className="secondary" onClick={()=>setShowCreate(false)}>Cancel</button><button className="primary" disabled={busy||!form.name||!form.slug}><Plus size={16}/> Create workspace</button></div>
      </form></div>}
    </main>
  </div>;
}
