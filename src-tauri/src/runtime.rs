use std::{env, path::PathBuf, process::Command};
use tauri::{AppHandle, Manager};

fn looks_like_runtime(path: &PathBuf) -> bool {
    path.join("php")
        .join(if cfg!(target_os = "windows") { "php.exe" } else { "php" })
        .exists()
        && path.join("wordpress").join("wp-settings.php").exists()
}

pub fn preferred_runtime_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve application data directory: {error}"))?
        .join("runtime"))
}

fn versions_root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve application data directory: {error}"))?
        .join("versions"))
}

pub fn runtime_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = preferred_runtime_dir(app) {
        candidates.push(path);
    }
    if let Ok(resource) = app.path().resource_dir() {
        candidates.push(resource.join("runtime"));
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("runtime"));
        }
    }
    candidates
}

pub fn resolve_runtime_dir(app: &AppHandle) -> Result<PathBuf, String> {
    runtime_candidates(app)
        .into_iter()
        .find(looks_like_runtime)
        .ok_or_else(|| {
            "PinWP Runtime Pack is not installed. Keep the runtime folder beside pinwp-studio.exe or install it into PinWP app data."
                .into()
        })
}

pub fn php_exe_for(app: &AppHandle, version: &str) -> Result<PathBuf, String> {
    if version.is_empty() || version == "Latest" || version == "8.4" {
        return Ok(resolve_runtime_dir(app)?
            .join("php")
            .join(if cfg!(target_os = "windows") { "php.exe" } else { "php" }));
    }

    let executable = versions_root(app)?
        .join("php")
        .join(version)
        .join(if cfg!(target_os = "windows") { "php.exe" } else { "php" });

    if executable.exists() {
        Ok(executable)
    } else {
        Err(format!(
            "PHP {version} is not installed. Install it from Tools → Runtime Versions."
        ))
    }
}

pub fn php_ini_for(app: &AppHandle, version: &str) -> Result<PathBuf, String> {
    if version.is_empty() || version == "Latest" || version == "8.4" {
        return Ok(resolve_runtime_dir(app)?.join("php").join("php.ini"));
    }

    let ini = versions_root(app)?
        .join("php")
        .join(version)
        .join("php.ini");

    if ini.exists() {
        Ok(ini)
    } else {
        Err(format!("PHP {version} configuration is not installed."))
    }
}

pub fn php_exe(app: &AppHandle) -> Result<PathBuf, String> {
    php_exe_for(app, "8.4")
}

pub fn php_ini(app: &AppHandle) -> Result<PathBuf, String> {
    php_ini_for(app, "8.4")
}

pub fn wordpress_dir_for(app: &AppHandle, version: &str) -> Result<PathBuf, String> {
    if version.is_empty() || version == "Latest" || version == "7.1" {
        return Ok(resolve_runtime_dir(app)?.join("wordpress"));
    }

    let directory = versions_root(app)?.join("wordpress").join(version);
    if directory.join("wp-settings.php").exists() {
        Ok(directory)
    } else {
        Err(format!(
            "WordPress {version} is not installed. Install it from Tools → Runtime Versions."
        ))
    }
}

pub fn wp_cli(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_runtime_dir(app)?.join("wp-cli").join("wp-cli.phar"))
}

pub fn mariadb_bin(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    Ok(resolve_runtime_dir(app)?
        .join("mariadb")
        .join("bin")
        .join(if cfg!(target_os = "windows") {
            format!("{name}.exe")
        } else {
            name.to_string()
        }))
}

pub fn cloudflared_exe(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_runtime_dir(app)?
        .join("cloudflared")
        .join(if cfg!(target_os = "windows") {
            "cloudflared.exe"
        } else {
            "cloudflared"
        }))
}

pub fn caddy_exe(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_runtime_dir(app)?
        .join("caddy")
        .join(if cfg!(target_os = "windows") { "caddy.exe" } else { "caddy" }))
}

pub fn mailpit_exe(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(resolve_runtime_dir(app)?
        .join("mailpit")
        .join(if cfg!(target_os = "windows") {
            "mailpit.exe"
        } else {
            "mailpit"
        }))
}

pub fn probe_bundled_php(app: &AppHandle) -> Option<String> {
    let output = Command::new(php_exe_for(app, "8.4").ok()?)
        .arg("-c")
        .arg(php_ini_for(app, "8.4").ok()?)
        .arg("-v")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::to_string)
}
