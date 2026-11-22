use crate::{
    models::{RuntimeVersionCatalog, RuntimeVersionItem, RuntimeVersionSource},
    runtime,
};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Manager};

#[derive(Debug, Deserialize)]
struct CatalogFile {
    php: Vec<RuntimeVersionSource>,
    wordpress: Vec<RuntimeVersionSource>,
}

fn versions_root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve application data directory: {error}"))?
        .join("versions"))
}

fn ps_quote(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

fn powershell(script: &str) -> Result<(), String> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .map_err(|error| format!("Unable to execute PowerShell: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

fn catalog_file(app: &AppHandle) -> Result<CatalogFile, String> {
    let path = runtime::resolve_runtime_dir(app)?.join("version-catalog.json");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Unable to read {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("Invalid runtime version catalog: {error}"))
}

fn installed_php(app: &AppHandle, key: &str) -> bool {
    runtime::php_exe_for(app, key)
        .map(|path| path.exists())
        .unwrap_or(false)
}

fn installed_wordpress(app: &AppHandle, key: &str) -> bool {
    runtime::wordpress_dir_for(app, key)
        .map(|path| path.join("wp-settings.php").exists())
        .unwrap_or(false)
}

pub fn list(app: &AppHandle) -> Result<RuntimeVersionCatalog, String> {
    let catalog = catalog_file(app)?;

    Ok(RuntimeVersionCatalog {
        php: catalog
            .php
            .into_iter()
            .map(|source| RuntimeVersionItem {
                bundled: source.key == "8.4",
                installed: installed_php(app, &source.key),
                key: source.key,
                version: source.version,
            })
            .collect(),
        wordpress: catalog
            .wordpress
            .into_iter()
            .map(|source| RuntimeVersionItem {
                bundled: source.key == "7.1",
                installed: installed_wordpress(app, &source.key),
                key: source.key,
                version: source.version,
            })
            .collect(),
    })
}

fn find_source(app: &AppHandle, kind: &str, key: &str) -> Result<RuntimeVersionSource, String> {
    let catalog = catalog_file(app)?;
    let entries = match kind {
        "php" => catalog.php,
        "wordpress" => catalog.wordpress,
        _ => return Err("Unknown runtime version kind.".into()),
    };

    entries
        .into_iter()
        .find(|entry| entry.key == key)
        .ok_or_else(|| format!("{kind} {key} is not in the verified version catalog."))
}

pub fn install(
    app: &AppHandle,
    kind: &str,
    key: &str,
) -> Result<RuntimeVersionCatalog, String> {
    let source = find_source(app, kind, key)?;

    if (kind == "php" && source.key == "8.4")
        || (kind == "wordpress" && source.key == "7.1")
    {
        return list(app);
    }

    let root = versions_root(app)?;
    let downloads = root.join("downloads");
    fs::create_dir_all(&downloads).map_err(|error| error.to_string())?;

    let archive = downloads.join(format!("{}-{}.zip", kind, key.replace('.', "_")));
    let destination = match kind {
        "php" => root.join("php").join(key),
        "wordpress" => root.join("wordpress").join(key),
        _ => return Err("Unknown runtime version kind.".into()),
    };

    if destination.exists() {
        fs::remove_dir_all(&destination).map_err(|error| error.to_string())?;
    }

    powershell(&format!(
        "Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
        source.url.replace('\'', "''"),
        ps_quote(&archive)
    ))?;

    if let Some(expected) = source.sha256.as_ref() {
        powershell(&format!(
            "$hash=(Get-FileHash -Algorithm SHA256 '{}').Hash.ToUpperInvariant(); if($hash -ne '{}'){{throw 'SHA256 mismatch: '+$hash}}",
            ps_quote(&archive),
            expected.to_uppercase()
        ))?;
    }

    let stage = root
        .join("stage")
        .join(format!("{}-{}", kind, key.replace('.', "_")));
    if stage.exists() {
        fs::remove_dir_all(&stage).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&stage).map_err(|error| error.to_string())?;

    powershell(&format!(
        "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
        ps_quote(&archive),
        ps_quote(&stage)
    ))?;

    if kind == "php" {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::rename(&stage, &destination)
            .map_err(|error| format!("Unable to install PHP {key}: {error}"))?;

        fs::copy(
            runtime::resolve_runtime_dir(app)?.join("php").join("php.ini"),
            destination.join("php.ini"),
        )
        .map_err(|error| format!("Unable to configure PHP {key}: {error}"))?;

        let check = Command::new(destination.join("php.exe"))
            .arg("-c")
            .arg(destination.join("php.ini"))
            .arg("-v")
            .output()
            .map_err(|error| error.to_string())?;
        if !check.status.success() {
            return Err(format!("PHP {key} failed its post-install check."));
        }
    } else {
        let extracted = stage.join("wordpress");
        if !extracted.join("wp-settings.php").exists() {
            return Err("WordPress archive layout is not recognized.".into());
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::rename(&extracted, &destination)
            .map_err(|error| format!("Unable to install WordPress {key}: {error}"))?;
        let _ = fs::remove_dir_all(&stage);
    }

    let _ = fs::remove_file(&archive);
    list(app)
}

pub fn remove(
    app: &AppHandle,
    kind: &str,
    key: &str,
) -> Result<RuntimeVersionCatalog, String> {
    if (kind == "php" && key == "8.4") || (kind == "wordpress" && key == "7.1") {
        return Err("The bundled default runtime version cannot be removed.".into());
    }

    let root = versions_root(app)?;
    let path = match kind {
        "php" => root.join("php").join(key),
        "wordpress" => root.join("wordpress").join(key),
        _ => return Err("Unknown runtime version kind.".into()),
    };

    if path.exists() {
        fs::remove_dir_all(path).map_err(|error| error.to_string())?;
    }

    list(app)
}
