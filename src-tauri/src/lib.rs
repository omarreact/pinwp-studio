mod models;mod processes;mod provision;mod registry;mod runtime;
use models::{CreateSiteInput,ProvisionResult,RuntimePreflight,SiteRecord,SiteStatus,ToolProbe};
use processes::ProcessRegistry;
use std::process::Command;
use tauri::{AppHandle,Manager,State};

fn first_line(bytes:&[u8])->Option<String>{String::from_utf8_lossy(bytes).lines().next().map(|s|s.trim().to_string()).filter(|s|!s.is_empty())}
fn probe(name:&str,exe:&str,args:&[&str])->ToolProbe{
    match Command::new(exe).args(args).output(){
        Ok(out) if out.status.success()=>ToolProbe{name:name.into(),available:true,version:first_line(&out.stdout).or_else(||first_line(&out.stderr)),source:Some("PATH".into())},
        _=>ToolProbe{name:name.into(),available:false,version:None,source:Some("Not detected on PATH".into())}
    }
}
#[tauri::command]
fn list_sites(app:AppHandle,state:State<'_,ProcessRegistry>)->Result<Vec<SiteRecord>,String>{
    let mut sites=registry::load_sites(&app)?;let mut changed=false;
    for site in &mut sites{
        let expected=if processes::is_running(&state,&site.id){SiteStatus::Running}else if site.status==SiteStatus::Provisioning{SiteStatus::Error}else{site.status.clone()};
        if site.status!=expected{site.status=expected;changed=true}
    }
    if changed{registry::save_sites(&app,&sites)?}Ok(sites)
}
#[tauri::command] fn create_site(app:AppHandle,input:CreateSiteInput)->Result<SiteRecord,String>{registry::create_site(&app,input)}
#[tauri::command] fn start_site(app:AppHandle,state:State<'_,ProcessRegistry>,id:String)->Result<SiteRecord,String>{processes::start(&app,&state,&id)}
#[tauri::command] fn stop_site(app:AppHandle,state:State<'_,ProcessRegistry>,id:String)->Result<SiteRecord,String>{processes::stop(&app,&state,&id)}
#[tauri::command] fn provision_site(app:AppHandle,id:String)->Result<ProvisionResult,String>{provision::provision(&app,&id)}
#[tauri::command]
fn runtime_preflight(app:AppHandle)->Result<RuntimePreflight,String>{
    let preferred=runtime::preferred_runtime_dir(&app)?;
    let app_data=app.path().app_data_dir().map_err(|e|format!("Unable to resolve application data directory: {e}"))?;
    let resolved=runtime::resolve_runtime_dir(&app).ok();
    let php=match runtime::probe_bundled_php(&app){
        Some(version)=>ToolProbe{name:"PHP".into(),available:true,version:Some(version),source:Some("PinWP Runtime Pack".into())},
        None=>probe("PHP","php",&["-v"])
    };
    let (cached_wordpress,cached_sqlite)=resolved.as_ref().map(|dir|(dir.join("wordpress").join("wp-settings.php").exists(),dir.join("plugins").join("sqlite-database-integration").join("db.copy").exists())).unwrap_or((false,false));
    Ok(RuntimePreflight{
        app_data_dir:app_data.to_string_lossy().into_owned(),
        runtime_pack_dir:resolved.unwrap_or(preferred).to_string_lossy().into_owned(),
        cached_wordpress,
        cached_sqlite_integration:cached_sqlite,
        tools:vec![php,probe("WP-CLI","wp",&["--info"]),probe("Git","git",&["--version"]),probe("Cloudflared","cloudflared",&["--version"])]
    })
}
#[cfg_attr(mobile,tauri::mobile_entry_point)]
pub fn run(){tauri::Builder::default().plugin(tauri_plugin_opener::init()).manage(ProcessRegistry::default()).invoke_handler(tauri::generate_handler![list_sites,create_site,start_site,stop_site,provision_site,runtime_preflight]).run(tauri::generate_context!()).expect("error while running PinWP Studio")}
