mod models;mod processes;mod registry;
use models::{CreateSiteInput,RuntimePreflight,SiteRecord,SiteStatus,ToolProbe};
use processes::ProcessRegistry;
use std::{path::Path,process::Command};
use tauri::{AppHandle,State};

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
    for site in &mut sites{let expected=if processes::is_running(&state,&site.id){SiteStatus::Running}else{SiteStatus::Stopped};if site.status!=expected{site.status=expected;changed=true}}
    if changed{registry::save_sites(&app,&sites)?}Ok(sites)
}
#[tauri::command] fn create_site(app:AppHandle,input:CreateSiteInput)->Result<SiteRecord,String>{registry::create_site(&app,input)}
#[tauri::command] fn start_site(app:AppHandle,state:State<'_,ProcessRegistry>,id:String)->Result<SiteRecord,String>{processes::start(&app,&state,&id)}
#[tauri::command] fn stop_site(app:AppHandle,state:State<'_,ProcessRegistry>,id:String)->Result<SiteRecord,String>{processes::stop(&app,&state,&id)}
#[tauri::command]
fn runtime_preflight(app:AppHandle)->Result<RuntimePreflight,String>{
    let runtime=registry::runtime_pack_dir(&app)?;let app_data=registry::sites_root(&app)?.parent().map(Path::to_path_buf).ok_or_else(||"Unable to resolve application data directory.".to_string())?;
    let bundled=runtime.join("php").join(if cfg!(target_os="windows"){"php.exe"}else{"php"});
    let php=if bundled.exists(){match Command::new(&bundled).arg("-v").output(){Ok(out) if out.status.success()=>ToolProbe{name:"PHP".into(),available:true,version:first_line(&out.stdout),source:Some("PinWP Runtime Pack".into())},_=>probe("PHP","php",&["-v"])}}else{probe("PHP","php",&["-v"])};
    Ok(RuntimePreflight{app_data_dir:app_data.to_string_lossy().into_owned(),runtime_pack_dir:runtime.to_string_lossy().into_owned(),cached_wordpress:runtime.join("wordpress").join("wordpress.zip").exists(),cached_sqlite_integration:runtime.join("wordpress").join("sqlite-database-integration.zip").exists(),tools:vec![php,probe("WP-CLI","wp",&["--info"]),probe("Git","git",&["--version"]),probe("Cloudflared","cloudflared",&["--version"])]})
}
#[cfg_attr(mobile,tauri::mobile_entry_point)]
pub fn run(){tauri::Builder::default().plugin(tauri_plugin_opener::init()).manage(ProcessRegistry::default()).invoke_handler(tauri::generate_handler![list_sites,create_site,start_site,stop_site,runtime_preflight]).run(tauri::generate_context!()).expect("error while running PinWP Studio")}
