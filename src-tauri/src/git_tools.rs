use crate::{models::GitInfo,registry};
use std::{path::PathBuf,process::Command};
use tauri::AppHandle;
fn run(dir:&PathBuf,args:&[&str])->Result<String,String>{let out=Command::new("git").args(args).current_dir(dir).output().map_err(|_|"Git is not installed or not available on PATH.".to_string())?;let stdout=String::from_utf8_lossy(&out.stdout).to_string();let stderr=String::from_utf8_lossy(&out.stderr).to_string();if out.status.success(){Ok(stdout)}else{Err(if stderr.trim().is_empty(){stdout}else{stderr})}}
pub fn status(app:&AppHandle,site_id:&str)->Result<GitInfo,String>{let site=registry::get_site(app,site_id)?;let dir=PathBuf::from(site.path).join("public");let available=Command::new("git").arg("--version").output().map(|o|o.status.success()).unwrap_or(false);if !available{return Ok(GitInfo{available:false,initialized:false,branch:None,dirty:false,output:"Git not detected.".into()})}if !dir.join(".git").exists(){return Ok(GitInfo{available:true,initialized:false,branch:None,dirty:false,output:"Repository is not initialized.".into()})}let output=run(&dir,&["status","--short","--branch"])?;let branch=run(&dir,&["branch","--show-current"]).ok().map(|s|s.trim().to_string()).filter(|s|!s.is_empty());Ok(GitInfo{available:true,initialized:true,branch,dirty:output.lines().any(|l|!l.starts_with("##")),output})}
pub fn init(app:&AppHandle,site_id:&str)->Result<GitInfo,String>{let site=registry::get_site(app,site_id)?;let dir=PathBuf::from(site.path).join("public");run(&dir,&["init"])?;
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(&ignore,
            "# PinWP Studio: local secrets and mutable runtime data\nwp-config.php\nwp-content/db.php\nwp-content/debug.log\nwp-content/uploads/\n*.sql\n*.sqlite\n*.sqlite3\n*.log\n.env\n.env.*\n"
        ).map_err(|e| format!("Unable to protect generated WordPress secrets: {e}"))?;
    }
    let _=run(&dir,&["branch","-M","main"]);status(app,site_id)}
pub fn commit(app:&AppHandle,site_id:&str,message:&str)->Result<GitInfo,String>{if message.trim().is_empty(){return Err("Commit message is required.".into())}let site=registry::get_site(app,site_id)?;let dir=PathBuf::from(site.path).join("public");run(&dir,&["add","-A"])?;run(&dir,&["commit","-m",message.trim()])?;status(app,site_id)}
pub fn open_code(app:&AppHandle,site_id:&str)->Result<(),String>{let site=registry::get_site(app,site_id)?;Command::new("code").arg(PathBuf::from(site.path).join("public")).spawn().map_err(|_|"VS Code 'code' command is not available on PATH.".to_string())?;Ok(())}
