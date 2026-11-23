mod database;mod deploy;mod git_tools;mod matrix;mod models;mod processes;mod provision;mod registry;mod router;mod runtime;mod tools;mod versions;mod workspace;
use models::{CompatibilityMatrixReport,CompatibilityReport,CreateSiteInput,DatabaseResult,DeployInput,DeployResult,GitInfo,LogRecord,ProvisionResult,RouterInfo,RuntimePreflight,RuntimeVersionCatalog,ShareInfo,SiteRecord,SiteStatus,SnapshotRecord,ToolProbe};
use processes::ProcessRegistry;
use std::process::Command;
use tauri::{AppHandle,Manager,State};

fn first_line(bytes:&[u8])->Option<String>{String::from_utf8_lossy(bytes).lines().next().map(|s|s.trim().to_string()).filter(|s|!s.is_empty())}
fn probe(name:&str,exe:&str,args:&[&str])->ToolProbe{match Command::new(exe).args(args).output(){Ok(out)if out.status.success()=>ToolProbe{name:name.into(),available:true,version:first_line(&out.stdout).or_else(||first_line(&out.stderr)),source:Some("PATH".into())},_=>ToolProbe{name:name.into(),available:false,version:None,source:Some("Not detected on PATH".into())}}}
#[tauri::command]fn list_sites(app:AppHandle,state:State<'_,ProcessRegistry>)->Result<Vec<SiteRecord>,String>{let mut sites=registry::load_sites(&app)?;let mut changed=false;for site in &mut sites{let expected=if processes::is_running(&state,&site.id){SiteStatus::Running}else if site.status==SiteStatus::Provisioning{SiteStatus::Error}else{site.status.clone()};if site.status!=expected{site.status=expected;changed=true}}if changed{registry::save_sites(&app,&sites)?}Ok(sites)}
#[tauri::command]fn create_site(app:AppHandle,input:CreateSiteInput)->Result<SiteRecord,String>{registry::create_site(&app,input)}
#[tauri::command]fn start_site(app:AppHandle,state:State<'_,ProcessRegistry>,id:String)->Result<SiteRecord,String>{processes::start(&app,&state,&id)}
#[tauri::command]fn stop_site(app:AppHandle,state:State<'_,ProcessRegistry>,id:String)->Result<SiteRecord,String>{processes::stop(&app,&state,&id)}
#[tauri::command]fn provision_site(app:AppHandle,state:State<'_,ProcessRegistry>,id:String)->Result<ProvisionResult,String>{provision::provision(&app,&state,&id)}
#[tauri::command]fn database_query(app:AppHandle,state:State<'_,ProcessRegistry>,id:String,sql:String,allow_write:bool)->Result<DatabaseResult,String>{database::query(&app,&state,&id,&sql,allow_write)}
#[tauri::command]fn list_snapshots(app:AppHandle,id:String)->Result<Vec<SnapshotRecord>,String>{workspace::list_snapshots(&app,&id)}
#[tauri::command]fn create_snapshot(app:AppHandle,state:State<'_,ProcessRegistry>,id:String,name:String)->Result<SnapshotRecord,String>{workspace::create_snapshot(&app,&state,&id,&name)}
#[tauri::command]fn restore_snapshot(app:AppHandle,state:State<'_,ProcessRegistry>,id:String,snapshot_id:String)->Result<(),String>{workspace::restore_snapshot(&app,&state,&id,&snapshot_id)}
#[tauri::command]fn delete_snapshot(app:AppHandle,id:String,snapshot_id:String)->Result<(),String>{workspace::delete_snapshot(&app,&id,&snapshot_id)}
#[tauri::command]fn export_site(app:AppHandle,state:State<'_,ProcessRegistry>,id:String)->Result<String,String>{workspace::export_site(&app,&state,&id)}
#[tauri::command]fn export_database_sql(app:AppHandle,state:State<'_,ProcessRegistry>,id:String)->Result<String,String>{workspace::export_database_sql(&app,&state,&id)}
#[tauri::command]fn import_site(app:AppHandle,state:State<'_,ProcessRegistry>,id:String,archive:String)->Result<(),String>{workspace::import_site(&app,&state,&id,&archive)}
#[tauri::command]fn list_logs(app:AppHandle,id:String)->Result<Vec<LogRecord>,String>{workspace::list_logs(&app,&id)}
#[tauri::command]fn read_log(app:AppHandle,path:String,max_bytes:usize)->Result<String,String>{workspace::read_log(&app,&path,max_bytes)}
#[tauri::command]fn git_status(app:AppHandle,id:String)->Result<GitInfo,String>{git_tools::status(&app,&id)}
#[tauri::command]fn git_init(app:AppHandle,id:String)->Result<GitInfo,String>{git_tools::init(&app,&id)}
#[tauri::command]fn git_commit(app:AppHandle,id:String,message:String)->Result<GitInfo,String>{git_tools::commit(&app,&id,&message)}
#[tauri::command]fn open_code(app:AppHandle,id:String)->Result<(),String>{git_tools::open_code(&app,&id)}
#[tauri::command]fn start_tunnel(app:AppHandle,state:State<'_,ProcessRegistry>,id:String)->Result<ShareInfo,String>{let site=registry::get_site(&app,&id)?;processes::start_tunnel(&app,&state,&site)}
#[tauri::command]fn start_named_tunnel(app:AppHandle,state:State<'_,ProcessRegistry>,id:String,token:String,hostname:String)->Result<ShareInfo,String>{let site=registry::get_site(&app,&id)?;processes::start_named_tunnel(&app,&state,&site,&token,&hostname)}
#[tauri::command]fn stop_tunnel(state:State<'_,ProcessRegistry>,id:String)->Result<(),String>{processes::stop_tunnel(&state,&id)}
#[tauri::command]fn start_mailpit(app:AppHandle,state:State<'_,ProcessRegistry>)->Result<String,String>{processes::start_mailpit(&app,&state)}
#[tauri::command]fn stop_mailpit(state:State<'_,ProcessRegistry>)->Result<(),String>{processes::stop_mailpit(&state)}
#[tauri::command]fn router_status(app:AppHandle,id:String)->Result<RouterInfo,String>{router::status(&app,&id)}
#[tauri::command]fn enable_router(app:AppHandle,id:String)->Result<RouterInfo,String>{router::enable(&app,&id)}
#[tauri::command]fn disable_router(app:AppHandle,id:String)->Result<RouterInfo,String>{router::disable(&app,&id)}
#[tauri::command]fn run_wp_cli(app:AppHandle,id:String,args:Vec<String>,allow_unsafe:bool)->Result<String,String>{tools::wp_cli(&app,&id,&args,allow_unsafe)}
#[tauri::command]fn compatibility_report(app:AppHandle,id:String)->Result<CompatibilityReport,String>{tools::compatibility(&app,&id)}
#[tauri::command]fn compatibility_matrix(app:AppHandle,state:State<'_,ProcessRegistry>,id:String)->Result<CompatibilityMatrixReport,String>{matrix::run(&app,&state,&id)}
#[tauri::command]fn test_deployment(input:DeployInput)->Result<DeployResult,String>{deploy::test(&input)}
#[tauri::command]fn deploy_site(app:AppHandle,state:State<'_,ProcessRegistry>,input:DeployInput)->Result<DeployResult,String>{deploy::deploy(&app,&state,&input)}
#[tauri::command]fn rollback_deployment(input:DeployInput)->Result<DeployResult,String>{deploy::rollback(&input)}
#[tauri::command]fn runtime_versions(app:AppHandle)->Result<RuntimeVersionCatalog,String>{versions::list(&app)}
#[tauri::command]fn install_runtime_version(app:AppHandle,kind:String,key:String)->Result<RuntimeVersionCatalog,String>{versions::install(&app,&kind,&key)}
#[tauri::command]fn remove_runtime_version(app:AppHandle,kind:String,key:String)->Result<RuntimeVersionCatalog,String>{versions::remove(&app,&kind,&key)}
#[tauri::command]fn runtime_preflight(app:AppHandle)->Result<RuntimePreflight,String>{let preferred=runtime::preferred_runtime_dir(&app)?;let app_data=app.path().app_data_dir().map_err(|e|format!("Unable to resolve application data directory: {e}"))?;let resolved=runtime::resolve_runtime_dir(&app).ok();let php=match runtime::probe_bundled_php(&app){Some(version)=>ToolProbe{name:"PHP".into(),available:true,version:Some(version),source:Some("PinWP Runtime Pack".into())},None=>probe("PHP","php",&["-v"])};let flags=resolved.as_ref().map(|d|(d.join("wordpress").join("wp-settings.php").exists(),d.join("plugins").join("sqlite-database-integration").join("db.copy").exists(),d.join("mariadb").join("bin").join("mariadbd.exe").exists(),d.join("cloudflared").join("cloudflared.exe").exists(),d.join("caddy").join("caddy.exe").exists(),d.join("mailpit").join("mailpit.exe").exists())).unwrap_or((false,false,false,false,false,false));Ok(RuntimePreflight{app_data_dir:app_data.to_string_lossy().into_owned(),runtime_pack_dir:resolved.unwrap_or(preferred).to_string_lossy().into_owned(),cached_wordpress:flags.0,cached_sqlite_integration:flags.1,cached_mariadb:flags.2,cached_cloudflared:flags.3,cached_caddy:flags.4,cached_mailpit:flags.5,tools:vec![php,probe("WP-CLI","wp",&["--info"]),probe("Git","git",&["--version"]),probe("SSH","ssh",&["-V"]),probe("Cloudflared","cloudflared",&["--version"])]})}
#[cfg_attr(mobile,tauri::mobile_entry_point)]pub fn run(){tauri::Builder::default().plugin(tauri_plugin_opener::init()).manage(ProcessRegistry::default()).invoke_handler(tauri::generate_handler![list_sites,create_site,start_site,stop_site,provision_site,database_query,list_snapshots,create_snapshot,restore_snapshot,delete_snapshot,export_site,export_database_sql,import_site,list_logs,read_log,git_status,git_init,git_commit,open_code,start_tunnel,start_named_tunnel,stop_tunnel,start_mailpit,stop_mailpit,router_status,enable_router,disable_router,run_wp_cli,compatibility_report,compatibility_matrix,test_deployment,deploy_site,rollback_deployment,runtime_versions,install_runtime_version,remove_runtime_version,runtime_preflight]).run(tauri::generate_context!()).expect("error while running PinWP Studio")}
