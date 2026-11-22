use crate::{models::{DatabaseMode,LogRecord,SnapshotRecord},processes::{self,ProcessRegistry},registry};
use chrono::Utc;
use std::{fs,path::{Path,PathBuf},process::Command,time::UNIX_EPOCH};
use tauri::{AppHandle,State};
use uuid::Uuid;

fn copy_tree(src:&Path,dst:&Path)->Result<(),String>{fs::create_dir_all(dst).map_err(|e|e.to_string())?;for e in fs::read_dir(src).map_err(|e|e.to_string())?{let e=e.map_err(|e|e.to_string())?;let s=e.path();let d=dst.join(e.file_name());if s.is_dir(){copy_tree(&s,&d)?}else{fs::copy(&s,&d).map_err(|e|e.to_string())?;}}Ok(())}
fn clear(path:&Path)->Result<(),String>{if !path.exists(){fs::create_dir_all(path).map_err(|e|e.to_string())?;return Ok(())}for e in fs::read_dir(path).map_err(|e|e.to_string())?{let p=e.map_err(|e|e.to_string())?.path();if p.is_dir(){fs::remove_dir_all(p)}else{fs::remove_file(p)}.map_err(|e|e.to_string())?;}Ok(())}
fn size(path:&Path)->u64{if path.is_file(){return fs::metadata(path).map(|m|m.len()).unwrap_or(0)}fs::read_dir(path).map(|it|it.filter_map(Result::ok).map(|e|size(&e.path())).sum()).unwrap_or(0)}
fn meta(path:&Path)->PathBuf{path.join("snapshot.json")}
pub fn list_snapshots(app:&AppHandle,site_id:&str)->Result<Vec<SnapshotRecord>,String>{let site=registry::get_site(app,site_id)?;let root=PathBuf::from(site.path).join("snapshots");let mut out=Vec::new();if root.exists(){for e in fs::read_dir(root).map_err(|e|e.to_string())?{let p=e.map_err(|e|e.to_string())?.path();if p.is_dir()&&meta(&p).exists(){if let Ok(v)=serde_json::from_str::<SnapshotRecord>(&fs::read_to_string(meta(&p)).unwrap_or_default()){out.push(v)}}}}out.sort_by(|a,b|b.created_at.cmp(&a.created_at));Ok(out)}
pub fn create_snapshot(app:&AppHandle,state:&State<'_,ProcessRegistry>,site_id:&str,name:&str)->Result<SnapshotRecord,String>{
    let mut site=registry::get_site(app,site_id)?;if processes::is_running(state,site_id){let _=processes::stop(app,state,site_id)?}
    let id=Uuid::new_v4().simple().to_string()[..12].to_string();let created=Utc::now().to_rfc3339();let root=PathBuf::from(&site.path).join("snapshots").join(format!("{}-{}",Utc::now().format("%Y%m%d-%H%M%S"),id));fs::create_dir_all(&root).map_err(|e|e.to_string())?;
    copy_tree(&PathBuf::from(&site.path).join("public"),&root.join("public"))?;
    match site.database_mode{DatabaseMode::Sqlite=>{let db=PathBuf::from(&site.path).join("private").join("database.sqlite");if db.exists(){fs::copy(db,root.join("database.sqlite")).map_err(|e|e.to_string())?;}},DatabaseMode::Mariadb=>processes::dump_mariadb(app,state,&site,&root.join("database.sql"))?}
    let record=SnapshotRecord{id,name:if name.trim().is_empty(){"Snapshot".into()}else{name.trim().into()},created_at:created.clone(),path:root.to_string_lossy().into_owned(),size_bytes:size(&root),database_mode:site.database_mode.clone()};fs::write(meta(&root),serde_json::to_string_pretty(&record).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;site.last_snapshot_at=Some(created);registry::update_site(app,&site)?;Ok(record)
}
pub fn restore_snapshot(app:&AppHandle,state:&State<'_,ProcessRegistry>,site_id:&str,snapshot_id:&str)->Result<(),String>{
    let site=registry::get_site(app,site_id)?;if processes::is_running(state,site_id){let _=processes::stop(app,state,site_id)?}
    let snap=list_snapshots(app,site_id)?.into_iter().find(|s|s.id==snapshot_id).ok_or("Snapshot not found.")?;let root=PathBuf::from(&snap.path);let public=PathBuf::from(&site.path).join("public");clear(&public)?;copy_tree(&root.join("public"),&public)?;
    match site.database_mode{DatabaseMode::Sqlite=>{if root.join("database.sqlite").exists(){fs::create_dir_all(PathBuf::from(&site.path).join("private")).map_err(|e|e.to_string())?;fs::copy(root.join("database.sqlite"),PathBuf::from(&site.path).join("private").join("database.sqlite")).map_err(|e|e.to_string())?;}},DatabaseMode::Mariadb=>processes::restore_mariadb(app,state,&site,&root.join("database.sql"))?}
    Ok(())
}
pub fn delete_snapshot(app:&AppHandle,site_id:&str,snapshot_id:&str)->Result<(),String>{let snap=list_snapshots(app,site_id)?.into_iter().find(|s|s.id==snapshot_id).ok_or("Snapshot not found.")?;fs::remove_dir_all(snap.path).map_err(|e|e.to_string())}
fn ps_quote(path:&Path)->String{path.to_string_lossy().replace(''',"''")}
pub fn export_site(app:&AppHandle,state:&State<'_,ProcessRegistry>,site_id:&str)->Result<String,String>{
    let site=registry::get_site(app,site_id)?;let root=PathBuf::from(&site.path);let staging=root.join("exports").join(format!("staging-{}",Uuid::new_v4().simple()));fs::create_dir_all(&staging).map_err(|e|e.to_string())?;copy_tree(&root.join("public"),&staging.join("public"))?;
    match site.database_mode{DatabaseMode::Sqlite=>{let db=root.join("private").join("database.sqlite");if db.exists(){fs::copy(db,staging.join("database.sqlite")).map_err(|e|e.to_string())?;}},DatabaseMode::Mariadb=>processes::dump_mariadb(app,state,&site,&staging.join("database.sql"))?}
    fs::write(staging.join("manifest.json"),serde_json::to_string_pretty(&site).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let archive=root.join("exports").join(format!("{}-{}.pinwp.zip",site.slug,Utc::now().format("%Y%m%d-%H%M%S")));let command=format!("Compress-Archive -Path '{}\\*' -DestinationPath '{}' -Force",ps_quote(&staging),ps_quote(&archive));let status=Command::new("powershell").args(["-NoProfile","-Command",&command]).status().map_err(|e|e.to_string())?;let _=fs::remove_dir_all(&staging);if !status.success(){return Err("Unable to create PinWP export archive.".into())}Ok(archive.to_string_lossy().into_owned())
}
pub fn import_site(app:&AppHandle,state:&State<'_,ProcessRegistry>,site_id:&str,archive:&str)->Result<(),String>{
    let site=registry::get_site(app,site_id)?;if processes::is_running(state,site_id){let _=processes::stop(app,state,site_id)?}let archive=PathBuf::from(archive);if !archive.exists(){return Err("Import archive does not exist.".into())}
    let temp=PathBuf::from(&site.path).join(".pinwp").join(format!("import-{}",Uuid::new_v4().simple()));fs::create_dir_all(&temp).map_err(|e|e.to_string())?;let command=format!("Expand-Archive -Path '{}' -DestinationPath '{}' -Force",ps_quote(&archive),ps_quote(&temp));let status=Command::new("powershell").args(["-NoProfile","-Command",&command]).status().map_err(|e|e.to_string())?;if !status.success(){return Err("Unable to extract import archive.".into())}
    let public=if temp.join("public").exists(){temp.join("public")}else{temp.clone()};let target=PathBuf::from(&site.path).join("public");clear(&target)?;copy_tree(&public,&target)?;
    if temp.join("database.sqlite").exists(){fs::copy(temp.join("database.sqlite"),PathBuf::from(&site.path).join("private").join("database.sqlite")).map_err(|e|e.to_string())?}
    if temp.join("database.sql").exists()&&site.database_mode==DatabaseMode::Mariadb{processes::restore_mariadb(app,state,&site,&temp.join("database.sql"))?}
    let _=fs::remove_dir_all(&temp);Ok(())
}
pub fn list_logs(app:&AppHandle,site_id:&str)->Result<Vec<LogRecord>,String>{let site=registry::get_site(app,site_id)?;let root=PathBuf::from(&site.path);let candidates=[root.join("logs").join("php-server.log"),root.join("logs").join("cloudflared.log"),root.join("public").join("wp-content").join("debug.log"),registry::services_root(app)?.join("mariadb.log"),registry::services_root(app)?.join("mailpit.log")];let mut out=Vec::new();for p in candidates{if let Ok(m)=fs::metadata(&p){let modified=m.modified().ok().and_then(|t|t.duration_since(UNIX_EPOCH).ok()).map(|d|d.as_secs().to_string());out.push(LogRecord{name:p.file_name().and_then(|v|v.to_str()).unwrap_or("log").into(),path:p.to_string_lossy().into_owned(),size_bytes:m.len(),modified_at:modified});}}Ok(out)}
pub fn read_log(path:&str,max_bytes:usize)->Result<String,String>{let data=fs::read(path).map_err(|e|e.to_string())?;let start=data.len().saturating_sub(max_bytes.max(1024).min(1024*1024));Ok(String::from_utf8_lossy(&data[start..]).to_string())}
