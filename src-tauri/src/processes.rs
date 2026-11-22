use crate::{models::{SiteRecord,SiteStatus},registry,runtime};
use std::{collections::HashMap,fs::{self,OpenOptions},path::PathBuf,process::{Child,Command,Stdio},sync::Mutex};
use tauri::{AppHandle,State};

#[derive(Default)]
pub struct ProcessRegistry{processes:Mutex<HashMap<String,Child>>}
impl Drop for ProcessRegistry{fn drop(&mut self){if let Ok(mut ps)=self.processes.lock(){for (_,child) in ps.iter_mut(){let _=child.kill();let _=child.wait();}}}}

fn php_command(app:&AppHandle)->Result<(PathBuf,Option<PathBuf>),String>{
    if let Ok(path)=runtime::php_exe(app){return Ok((path,Some(runtime::php_ini(app)?)))}
    let out=Command::new("php").arg("-v").output().map_err(|_|"PHP was not detected. Install/extract a PinWP Runtime Pack or provide PHP on PATH.".to_string())?;
    if !out.status.success(){return Err("PHP exists but failed its version check.".into())}
    Ok((PathBuf::from("php"),None))
}

pub fn is_running(state:&State<'_,ProcessRegistry>,id:&str)->bool{state.processes.lock().map(|p|p.contains_key(id)).unwrap_or(false)}
pub fn start(app:&AppHandle,state:&State<'_,ProcessRegistry>,id:&str)->Result<SiteRecord,String>{
    if is_running(state,id){return registry::get_site(app,id)}
    let mut site=registry::get_site(app,id)?;
    let (php,ini)=php_command(app)?;
    let root=PathBuf::from(&site.path);let public=root.join("public");let logs=root.join("logs");let pinwp=root.join(".pinwp");
    fs::create_dir_all(&logs).map_err(|e|format!("Unable to create log directory: {e}"))?;
    fs::create_dir_all(&pinwp).map_err(|e|format!("Unable to create PinWP metadata directory: {e}"))?;
    let router=pinwp.join("router.php");
    let router_source=r#"<?php
$path = urldecode(parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH) ?? '/');
$file = rtrim($_SERVER['DOCUMENT_ROOT'], '/\\') . $path;
if ($path !== '/' && is_file($file)) {
    return false;
}
require $_SERVER['DOCUMENT_ROOT'] . '/index.php';
"#;
    fs::write(&router,router_source).map_err(|e|format!("Unable to write local WordPress router: {e}"))?;
    let log_path=logs.join("php-server.log");
    let stdout=OpenOptions::new().create(true).append(true).open(&log_path).map_err(|e|format!("Unable to open {}: {e}",log_path.display()))?;
    let stderr=stdout.try_clone().map_err(|e|format!("Unable to clone PHP log handle: {e}"))?;
    let addr=format!("127.0.0.1:{}",site.port);let public_string=public.to_string_lossy().into_owned();let router_string=router.to_string_lossy().into_owned();
    let mut command=Command::new(php);
    if let Some(ini)=ini{command.arg("-c").arg(ini);}
    let child=command.args(["-S",&addr,"-t",&public_string,&router_string]).current_dir(&public).stdout(Stdio::from(stdout)).stderr(Stdio::from(stderr)).spawn().map_err(|e|format!("Unable to start local PHP server: {e}"))?;
    state.processes.lock().map_err(|_|"PinWP process registry is unavailable.".to_string())?.insert(site.id.clone(),child);
    site.status=SiteStatus::Running;registry::update_site(app,&site)?;Ok(site)
}
pub fn stop(app:&AppHandle,state:&State<'_,ProcessRegistry>,id:&str)->Result<SiteRecord,String>{
    if let Ok(mut ps)=state.processes.lock(){if let Some(mut child)=ps.remove(id){let _=child.kill();let _=child.wait();}}
    let mut site=registry::get_site(app,id)?;site.status=SiteStatus::Stopped;registry::update_site(app,&site)?;Ok(site)
}
