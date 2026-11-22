use std::{env, path::PathBuf, process::Command};
use tauri::{AppHandle, Manager};

fn looks_like_runtime(path: &PathBuf) -> bool {
    path.join("php").join(if cfg!(target_os = "windows") { "php.exe" } else { "php" }).exists()
        && path.join("wordpress").join("wp-settings.php").exists()
}

pub fn preferred_runtime_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data = app.path().app_data_dir()
        .map_err(|error| format!("Unable to resolve application data directory: {error}"))?
        .join("runtime");
    Ok(app_data)
}

pub fn runtime_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = preferred_runtime_dir(app) { candidates.push(path); }
    if let Ok(resource) = app.path().resource_dir() { candidates.push(resource.join("runtime")); }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() { candidates.push(parent.join("runtime")); }
    }
    candidates
}

pub fn resolve_runtime_dir(app: &AppHandle) -> Result<PathBuf, String> {
    runtime_candidates(app)
        .into_iter()
        .find(looks_like_runtime)
        .ok_or_else(|| "PinWP Runtime Pack is not installed. Place the runtime folder beside PinWP Studio or install it into the PinWP application data directory.".into())
}

pub fn php_exe(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_runtime_dir(app)?.join("php").join(if cfg!(target_os = "windows") { "php.exe" } else { "php" }))
}

pub fn php_ini(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_runtime_dir(app)?.join("php").join("php.ini"))
}

pub fn wp_cli(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_runtime_dir(app)?.join("wp-cli").join("wp-cli.phar"))
}

pub fn probe_bundled_php(app: &AppHandle) -> Option<String> {
    let php = php_exe(app).ok()?;
    let ini = php_ini(app).ok()?;
    let output = Command::new(php).arg("-c").arg(ini).arg("-v").output().ok()?;
    if !output.status.success() { return None; }
    String::from_utf8_lossy(&output.stdout).lines().next().map(str::to_string)
}
