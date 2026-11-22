use crate::{models::GitInfo,registry};
use std::{fs,path::PathBuf,process::Command};
use tauri::AppHandle;
fn run(dir:&PathBuf,args:&[&str])->Result<String,String>{let out=Command::new("git").args(args).current_dir(dir).output().map_err(|_|"Git is not installed or not available on PATH.".to_string())?;let stdout=String::from_utf8_lossy(&out.stdout).to_string();let stderr=String::from_utf8_lossy(&out.stderr).to_string();if out.status.success(){Ok(stdout)}else{Err(if stderr.trim().is_empty(){stdout}else{stderr})}}
pub fn status(app:&AppHandle,site_id:&str)->Result<GitInfo,String>{let site=registry::get_site(app,site_id)?;let dir=PathBuf::from(site.path).join("public");let available=Command::new("git").arg("--version").output().map(|o|o.status.success()).unwrap_or(false);if !available{return Ok(GitInfo{available:false,initialized:false,branch:None,dirty:false,output:"Git not detected.".into()})}if !dir.join(".git").exists(){return Ok(GitInfo{available:true,initialized:false,branch:None,dirty:false,output:"Repository is not initialized.".into()})}let output=run(&dir,&["status","--short","--branch"])?;let branch=run(&dir,&["branch","--show-current"]).ok().map(|s|s.trim().to_string()).filter(|s|!s.is_empty());Ok(GitInfo{available:true,initialized:true,branch,dirty:output.lines().any(|l|!l.starts_with("##")),output})}
pub fn init(app:&AppHandle,site_id:&str)->Result<GitInfo,String>{let site=registry::get_site(app,site_id)?;let dir=PathBuf::from(site.path).join("public");run(&dir,&["init"])?;ensure_ignore(&dir)?;let _=run(&dir,&["branch","-M","main"]);status(app,site_id)}
pub fn commit(app:&AppHandle,site_id:&str,message:&str)->Result<GitInfo,String>{if message.trim().is_empty(){return Err("Commit message is required.".into())}let site=registry::get_site(app,site_id)?;let dir=PathBuf::from(site.path).join("public");ensure_ignore(&dir)?;let tracked=run(&dir,&["ls-files"])?;if tracked.lines().any(is_sensitive){return Err("Sensitive WordPress files are already tracked. Untrack them before committing.".into())}run(&dir,&["add","-A"])?;run(&dir,&["commit","-m",message.trim()])?;status(app,site_id)}
pub fn open_code(app:&AppHandle,site_id:&str)->Result<(),String>{let site=registry::get_site(app,site_id)?;Command::new("code").arg(PathBuf::from(site.path).join("public")).spawn().map_err(|_|"VS Code 'code' command is not available on PATH.".to_string())?;Ok(())}

const DEFAULT_IGNORE: &str = "\n# PinWP: never commit local credentials or generated data\n/wp-config.php\n/wp-content/db.php\n/wp-content/debug.log\n/wp-content/cache/\n/wp-content/uploads/\n*.sqlite\n*.sql\n.env\n.env.*\n";
fn ensure_ignore(dir:&PathBuf)->Result<(),String>{
    let path=dir.join(".gitignore");
    let current=fs::read_to_string(&path).unwrap_or_default();
    if current.contains("# PinWP: never commit local credentials") { return Ok(()); }
    fs::write(&path,format!("{current}{DEFAULT_IGNORE}")).map_err(|e|e.to_string())
}
fn is_sensitive(path:&str)->bool{
    let p=path.trim().replace('\\',"/").to_ascii_lowercase();
    p=="wp-config.php" || p=="wp-content/db.php" || p.starts_with("wp-content/uploads/") ||
    p.ends_with(".sqlite") || p.ends_with(".sql") || p==".env" || p.starts_with(".env.")
}
