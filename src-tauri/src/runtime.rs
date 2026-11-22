use std::{env, path::PathBuf, process::Command};
use tauri::{AppHandle, Manager};

fn looks_like_runtime(path:&PathBuf)->bool{path.join("php").join(if cfg!(target_os="windows"){"php.exe"}else{"php"}).exists()&&path.join("wordpress").join("wp-settings.php").exists()}
pub fn preferred_runtime_dir(app:&AppHandle)->Result<PathBuf,String>{Ok(app.path().app_data_dir().map_err(|e|format!("Unable to resolve application data directory: {e}"))?.join("runtime"))}
pub fn runtime_candidates(app:&AppHandle)->Vec<PathBuf>{let mut c=Vec::new();if let Ok(p)=preferred_runtime_dir(app){c.push(p)}if let Ok(r)=app.path().resource_dir(){c.push(r.join("runtime"))}if let Ok(exe)=env::current_exe(){if let Some(parent)=exe.parent(){c.push(parent.join("runtime"))}}c}
pub fn resolve_runtime_dir(app:&AppHandle)->Result<PathBuf,String>{runtime_candidates(app).into_iter().find(looks_like_runtime).ok_or_else(||"PinWP Runtime Pack is not installed. Keep the runtime folder beside pinwp-studio.exe or install it into PinWP app data.".into())}
pub fn php_exe(app:&AppHandle)->Result<PathBuf,String>{Ok(resolve_runtime_dir(app)?.join("php").join(if cfg!(target_os="windows"){"php.exe"}else{"php"}))}
pub fn php_ini(app:&AppHandle)->Result<PathBuf,String>{Ok(resolve_runtime_dir(app)?.join("php").join("php.ini"))}
pub fn wp_cli(app:&AppHandle)->Result<PathBuf,String>{Ok(resolve_runtime_dir(app)?.join("wp-cli").join("wp-cli.phar"))}
pub fn mariadb_bin(app:&AppHandle,name:&str)->Result<PathBuf,String>{Ok(resolve_runtime_dir(app)?.join("mariadb").join("bin").join(if cfg!(target_os="windows"){format!("{name}.exe")}else{name.to_string()}))}
pub fn cloudflared_exe(app:&AppHandle)->Result<PathBuf,String>{Ok(resolve_runtime_dir(app)?.join("cloudflared").join(if cfg!(target_os="windows"){"cloudflared.exe"}else{"cloudflared"}))}
pub fn caddy_exe(app:&AppHandle)->Result<PathBuf,String>{Ok(resolve_runtime_dir(app)?.join("caddy").join(if cfg!(target_os="windows"){"caddy.exe"}else{"caddy"}))}
pub fn mailpit_exe(app:&AppHandle)->Result<PathBuf,String>{Ok(resolve_runtime_dir(app)?.join("mailpit").join(if cfg!(target_os="windows"){"mailpit.exe"}else{"mailpit"}))}
pub fn probe_bundled_php(app:&AppHandle)->Option<String>{let output=Command::new(php_exe(app).ok()?).arg("-c").arg(php_ini(app).ok()?).arg("-v").output().ok()?;if !output.status.success(){return None}String::from_utf8_lossy(&output.stdout).lines().next().map(str::to_string)}
