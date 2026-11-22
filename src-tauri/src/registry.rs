use crate::models::{CreateSiteInput, SiteRecord, SiteStatus};
use chrono::Utc;
use std::{fs, path::{Path, PathBuf}};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

fn app_data_dir(app:&AppHandle)->Result<PathBuf,String>{
    app.path().app_data_dir().map_err(|e|format!("Unable to resolve PinWP application data directory: {e}"))
}
pub fn runtime_pack_dir(app:&AppHandle)->Result<PathBuf,String>{Ok(app_data_dir(app)?.join("runtime"))}
pub fn registry_path(app:&AppHandle)->Result<PathBuf,String>{Ok(app_data_dir(app)?.join("sites.json"))}
pub fn sites_root(app:&AppHandle)->Result<PathBuf,String>{Ok(app_data_dir(app)?.join("sites"))}

pub fn load_sites(app:&AppHandle)->Result<Vec<SiteRecord>,String>{
    let path=registry_path(app)?;
    if !path.exists(){return Ok(Vec::new())}
    let data=fs::read_to_string(&path).map_err(|e|format!("Unable to read site registry {}: {e}",path.display()))?;
    serde_json::from_str(&data).map_err(|e|format!("Unable to parse site registry {}: {e}",path.display()))
}
pub fn save_sites(app:&AppHandle,sites:&[SiteRecord])->Result<(),String>{
    let path=registry_path(app)?;
    if let Some(parent)=path.parent(){fs::create_dir_all(parent).map_err(|e|format!("Unable to create {}: {e}",parent.display()))?}
    let payload=serde_json::to_string_pretty(sites).map_err(|e|format!("Unable to serialize site registry: {e}"))?;
    fs::write(&path,payload).map_err(|e|format!("Unable to write site registry {}: {e}",path.display()))
}
fn sanitize_slug(value:&str)->String{
    let mut out=String::new();let mut dash=false;
    for c in value.trim().to_lowercase().chars(){
        if c.is_ascii_alphanumeric(){out.push(c);dash=false}else if !dash&&!out.is_empty(){out.push('-');dash=true}
    }
    out.trim_matches('-').chars().take(48).collect()
}
fn allocate_port(sites:&[SiteRecord])->u16{(9080..9999).find(|p|sites.iter().all(|s|s.port!=*p)).unwrap_or(9999)}
fn write_placeholder(public:&Path,name:&str)->Result<(),String>{
    let index=public.join("index.php");if index.exists(){return Ok(())}
    let safe=name.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;");
    let content=format!(r#"<?php header('Content-Type: text/html; charset=UTF-8'); ?><!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{0} · PinWP Studio</title></head><body style="font-family:system-ui;background:#0b0d12;color:#f6f7fb;margin:0;min-height:100vh;display:grid;place-items:center"><main style="max-width:620px;padding:40px;text-align:center"><div style="font-size:14px;color:#69e4a6">PinWP Studio</div><h1 style="font-size:34px">{0}</h1><p style="color:#9aa4b5;line-height:1.7">The local PHP server is working. WordPress has not been provisioned into this workspace yet. Runtime Packs will install WordPress core and the selected database adapter here.</p></main></body></html>"#,safe);
    fs::write(index,content).map_err(|e|format!("Unable to create workspace placeholder: {e}"))
}
pub fn create_site(app:&AppHandle,input:CreateSiteInput)->Result<SiteRecord,String>{
    let name=input.name.trim();if name.is_empty(){return Err("Site name is required.".into())}
    let slug=sanitize_slug(if input.slug.trim().is_empty(){name}else{&input.slug});
    if slug.is_empty(){return Err("Site slug must contain letters or numbers.".into())}
    let mut sites=load_sites(app)?;if sites.iter().any(|s|s.slug==slug){return Err(format!("A site using the slug '{slug}' already exists."))}
    let root=sites_root(app)?.join(&slug);let public=root.join("public");
    for folder in ["public","logs","snapshots","exports","private",".pinwp"]{fs::create_dir_all(root.join(folder)).map_err(|e|format!("Unable to create site workspace: {e}"))?}
    write_placeholder(&public,name)?;
    let site=SiteRecord{id:Uuid::new_v4().to_string(),name:name.to_string(),slug:slug.clone(),status:SiteStatus::Stopped,php_version:input.php_version,wordpress_version:input.wordpress_version,database_mode:input.database_mode,port:allocate_port(&sites),domain:format!("{slug}.localhost"),path:root.to_string_lossy().into_owned(),provisioned:false,created_at:Utc::now().to_rfc3339()};
    let metadata=serde_json::to_string_pretty(&site).map_err(|e|format!("Unable to serialize site metadata: {e}"))?;
    fs::write(root.join(".pinwp").join("site.json"),metadata).map_err(|e|format!("Unable to write site metadata: {e}"))?;
    sites.insert(0,site.clone());save_sites(app,&sites)?;Ok(site)
}
pub fn update_site(app:&AppHandle,updated:&SiteRecord)->Result<(),String>{
    let mut sites=load_sites(app)?;let Some(site)=sites.iter_mut().find(|s|s.id==updated.id) else{return Err("Site not found.".into())};*site=updated.clone();save_sites(app,&sites)
}
pub fn get_site(app:&AppHandle,id:&str)->Result<SiteRecord,String>{load_sites(app)?.into_iter().find(|s|s.id==id).ok_or_else(||"Site not found.".into())}
