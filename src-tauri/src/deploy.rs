use crate::{
    models::{DeployInput, DeployResult},
    processes::ProcessRegistry,
    registry, workspace,
};
use std::process::Command;
use tauri::{AppHandle, State};

fn run(executable: &str, args: &[String]) -> Result<String, String> {
    let output = Command::new(executable)
        .args(args)
        .output()
        .map_err(|error| format!("Unable to execute {executable}: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        Ok(if stdout.trim().is_empty() { stderr } else { stdout })
    } else {
        Err(if stderr.trim().is_empty() { stdout } else { stderr })
    }
}

fn validate(input: &DeployInput) -> Result<(), String> {
    let host = input.host.trim();
    let user = input.username.trim();
    let remote = input.remote_path.trim();

    if host.is_empty() || user.is_empty() {
        return Err("Host and username are required.".into());
    }
    if !host
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'))
    {
        return Err("Host contains unsupported characters.".into());
    }
    if !user
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@'))
    {
        return Err("Username contains unsupported characters.".into());
    }
    if remote.is_empty()
        || remote.contains("..")
        || !remote
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '\\' | '.' | '_' | '-'))
    {
        return Err("Remote path is invalid or unsafe.".into());
    }
    if input.kind == "connector"
        && !(input.site_url.starts_with("https://") || input.site_url.starts_with("http://"))
    {
        return Err("Connector site URL must start with http:// or https://.".into());
    }
    Ok(())
}

pub fn test(input: &DeployInput) -> Result<DeployResult, String> {
    validate(input)?;
    let message = match input.kind.as_str() {
        "ssh" => run(
            "ssh",
            &[
                "-p".into(),
                input.port.unwrap_or(22).to_string(),
                format!("{}@{}", input.username, input.host),
                "printf PINWP_OK".into(),
            ],
        )?,
        "cpanel" => run(
            "curl.exe",
            &[
                "-fsS".into(),
                "-H".into(),
                format!("Authorization: cpanel {}:{}", input.username, input.secret),
                format!(
                    "https://{}:{}/execute/Variables/get_server_information",
                    input.host,
                    input.port.unwrap_or(2083)
                ),
            ],
        )?,
        "connector" => run(
            "curl.exe",
            &[
                "-fsS".into(),
                "-u".into(),
                format!("{}:{}", input.username, input.secret),
                format!(
                    "{}/wp-json/pinwp/v1/health",
                    input.site_url.trim_end_matches('/')
                ),
            ],
        )?,
        _ => return Err("Unknown deployment adapter.".into()),
    };

    Ok(DeployResult {
        success: true,
        stage: "connection-test".into(),
        message,
        artifact_path: None,
    })
}

pub fn deploy(
    app: &AppHandle,
    state: &State<'_, ProcessRegistry>,
    input: &DeployInput,
) -> Result<DeployResult, String> {
    validate(input)?;
    let site = registry::get_site(app, &input.site_id)?;
    let archive = workspace::export_site(app, state, &input.site_id)?;
    let local_url = format!("http://{}:{}", site.domain, site.port);
    let remote_url = input.site_url.trim_end_matches('/');

    let message = match input.kind.as_str() {
        "ssh" => {
            if remote_url.is_empty() {
                return Err("Remote site URL is required for SSH database migration.".into());
            }
            let target = format!("{}@{}", input.username, input.host);
            let port = input.port.unwrap_or(22).to_string();
            let remote = input.remote_path.trim_end_matches('/');

            run(
                "ssh",
                &[
                    "-p".into(),
                    port.clone(),
                    target.clone(),
                    format!(
                        "set -e; stamp=$(date +%Y%m%d-%H%M%S); mkdir -p '{remote}/.pinwp-backups'; tar --exclude='./.pinwp-backups' -czf '{remote}/.pinwp-backups/pre-pinwp-'$stamp'.tgz' -C '{remote}' .; if command -v wp >/dev/null 2>&1 && [ -f '{remote}/wp-config.php' ]; then wp db export '{remote}/.pinwp-backups/pre-pinwp-'$stamp'.sql' --path='{remote}'; fi"
                    ),
                ],
            )?;

            run(
                "scp",
                &[
                    "-P".into(),
                    port.clone(),
                    archive.clone(),
                    format!("{target}:{remote}/pinwp-deploy.zip"),
                ],
            )?;

            run(
                "ssh",
                &[
                    "-p".into(),
                    port,
                    target,
                    format!(
                        "set -e; cd '{remote}'; rm -rf .pinwp-deploy; mkdir .pinwp-deploy; unzip -oq pinwp-deploy.zip -d .pinwp-deploy; test -d .pinwp-deploy/public; if [ -f wp-config.php ]; then cp wp-config.php .pinwp-production-config.php; fi; cp -a .pinwp-deploy/public/. .; if [ -f .pinwp-production-config.php ]; then mv .pinwp-production-config.php wp-config.php; fi; rm -f wp-content/db.php; if [ -f .pinwp-deploy/database.sql ]; then command -v wp >/dev/null 2>&1 || (echo 'WP-CLI is required for database migration' >&2; exit 42); wp db import .pinwp-deploy/database.sql --path='{remote}'; wp search-replace '{local_url}' '{remote_url}' --all-tables --precise --skip-columns=guid --path='{remote}'; fi; rm -rf .pinwp-deploy pinwp-deploy.zip"
                    ),
                ],
            )?
        }
        "cpanel" => run(
            "curl.exe",
            &[
                "-fsS".into(),
                "-H".into(),
                format!("Authorization: cpanel {}:{}", input.username, input.secret),
                "-F".into(),
                format!("dir={}", input.remote_path),
                "-F".into(),
                format!("file-1=@{archive}"),
                format!(
                    "https://{}:{}/execute/Fileman/upload_files",
                    input.host,
                    input.port.unwrap_or(2083)
                ),
            ],
        )?,
        "connector" => run(
            "curl.exe",
            &[
                "-fsS".into(),
                "-u".into(),
                format!("{}:{}", input.username, input.secret),
                "-F".into(),
                format!("package=@{archive}"),
                format!(
                    "{}/wp-json/pinwp/v1/deploy",
                    input.site_url.trim_end_matches('/')
                ),
            ],
        )?,
        _ => return Err("Unknown deployment adapter.".into()),
    };

    Ok(DeployResult {
        success: true,
        stage: "deploy".into(),
        message,
        artifact_path: Some(archive),
    })
}

pub fn rollback(input: &DeployInput) -> Result<DeployResult, String> {
    validate(input)?;
    let message = match input.kind.as_str() {
        "ssh" => {
            let target = format!("{}@{}", input.username, input.host);
            let port = input.port.unwrap_or(22).to_string();
            let remote = input.remote_path.trim_end_matches('/');
            run(
                "ssh",
                &[
                    "-p".into(),
                    port,
                    target,
                    format!(
                        "set -e; latest=$(ls -1t '{remote}/.pinwp-backups'/pre-pinwp-*.tgz 2>/dev/null | head -1); test -n \"$latest\"; stem=$(printf '%s' \"$latest\" | sed 's/\\.tgz$//'); find '{remote}' -mindepth 1 -maxdepth 1 ! -name .pinwp-backups -exec rm -rf {{}} +; tar -xzf \"$latest\" -C '{remote}'; if [ -f \"$stem.sql\" ] && command -v wp >/dev/null 2>&1; then wp db import \"$stem.sql\" --path='{remote}'; fi; echo \"Restored $latest\""
                    ),
                ],
            )?
        }
        "connector" => run(
            "curl.exe",
            &[
                "-fsS".into(),
                "-X".into(),
                "POST".into(),
                "-u".into(),
                format!("{}:{}", input.username, input.secret),
                format!(
                    "{}/wp-json/pinwp/v1/rollback",
                    input.site_url.trim_end_matches('/')
                ),
            ],
        )?,
        "cpanel" => {
            return Err(
                "Automatic cPanel rollback is not enabled because cPanel restore capabilities vary by host. Use the generated package/backup or PinWP Connector."
                    .into(),
            )
        }
        _ => return Err("Unknown deployment adapter.".into()),
    };

    Ok(DeployResult {
        success: true,
        stage: "rollback".into(),
        message,
        artifact_path: None,
    })
}
