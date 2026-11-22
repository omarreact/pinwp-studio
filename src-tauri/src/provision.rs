use crate::{
    models::{DatabaseCredentials, DatabaseMode, ProvisionResult, SiteStatus},
    processes::{self, ProcessRegistry},
    registry, runtime,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, State};
use uuid::Uuid;

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path)
                .map_err(|error| format!("Unable to copy {}: {error}", source_path.display()))?;
        }
    }
    Ok(())
}

fn clear_directory(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|error| error.to_string())?;
        return Ok(());
    }
    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        }
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn php_escape(value: &str) -> String {
    value.replace('\\', "/").replace('\'', "\\'")
}

fn write_config(
    public: &Path,
    database_mode: DatabaseMode,
    sqlite_path: Option<&Path>,
    maria: Option<&DatabaseCredentials>,
) -> Result<(), String> {
    let mut config = fs::read_to_string(public.join("wp-config-sample.php"))
        .map_err(|error| format!("Unable to read wp-config-sample.php: {error}"))?;

    match database_mode {
        DatabaseMode::Sqlite => {
            config = config
                .replace("database_name_here", "pinwp")
                .replace("username_here", "pinwp")
                .replace("password_here", "pinwp")
                .replace("localhost", "localhost");
        }
        DatabaseMode::Mariadb => {
            let credentials = maria.ok_or("MariaDB credentials are missing.")?;
            config = config
                .replace("database_name_here", &credentials.database)
                .replace("username_here", &credentials.username)
                .replace("password_here", &credentials.password)
                .replace(
                    "localhost",
                    &format!("{}:{}", credentials.host, credentials.port),
                );
        }
    }

    for key in [
        "AUTH_KEY", "SECURE_AUTH_KEY", "LOGGED_IN_KEY", "NONCE_KEY",
        "AUTH_SALT", "SECURE_AUTH_SALT", "LOGGED_IN_SALT", "NONCE_SALT",
    ] {
        let placeholder = format!("define( '{}', 'put your unique phrase here' );", key);
        let value = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
        config = config.replace(&placeholder, &format!("define( '{}', '{}' );", key, value));
    }

    config = config.replace(
        "define( 'WP_DEBUG', false );",
        "define( 'WP_DEBUG', true );\ndefine( 'WP_DEBUG_LOG', true );\ndefine( 'WP_DEBUG_DISPLAY', false );",
    );

    let anchor = "/* That's all, stop editing!";
    if !config.contains(anchor) {
        return Err("Unable to locate wp-config.php insertion anchor.".into());
    }

    let mut local = String::from(
        "define( 'WP_ENVIRONMENT_TYPE', 'local' );\n\
if ( isset( $_SERVER['HTTP_HOST'] ) ) {\n\
    $pinwp_scheme = ( ! empty( $_SERVER['HTTPS'] ) && $_SERVER['HTTPS'] !== 'off' ) ? 'https' : 'http';\n\
    define( 'WP_HOME', $pinwp_scheme . '://' . $_SERVER['HTTP_HOST'] );\n\
    define( 'WP_SITEURL', $pinwp_scheme . '://' . $_SERVER['HTTP_HOST'] );\n\
}\n",
    );

    if let Some(path) = sqlite_path {
        local = format!(
            "define( 'DB_PATH', '{}' );\n{}",
            php_escape(&path.to_string_lossy()),
            local
        );
    }

    config = config.replacen(anchor, &format!("{local}{anchor}"), 1);
    fs::write(public.join("wp-config.php"), config)
        .map_err(|error| format!("Unable to write wp-config.php: {error}"))
}

pub fn run_wp_version(
    app: &AppHandle,
    public: &Path,
    args: &[String],
    php_version: &str,
) -> Result<String, String> {
    let output = Command::new(runtime::php_exe_for(app, php_version)?)
        .arg("-c")
        .arg(runtime::php_ini_for(app, php_version)?)
        .arg(runtime::wp_cli(app)?)
        .arg(format!("--path={}", public.to_string_lossy()))
        .args(args)
        .output()
        .map_err(|error| format!("Unable to execute WP-CLI: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        return Err(format!(
            "WP-CLI failed: {}",
            if stderr.is_empty() { stdout } else { stderr }
        ));
    }
    Ok(stdout)
}

pub fn provision(
    app: &AppHandle,
    state: &State<'_, ProcessRegistry>,
    id: &str,
) -> Result<ProvisionResult, String> {
    let mut site = registry::get_site(app, id)?;
    if site.provisioned {
        return Err("This site is already provisioned.".into());
    }

    let runtime_dir = runtime::resolve_runtime_dir(app)?;
    let wordpress = runtime::wordpress_dir_for(app, &site.wordpress_version)?;
    let php = runtime::php_exe_for(app, &site.php_version)?;
    let wp_cli = runtime::wp_cli(app)?;

    for required in [wordpress.join("wp-settings.php"), php, wp_cli] {
        if !required.exists() {
            return Err(format!("Runtime component is missing: {}", required.display()));
        }
    }

    let root = PathBuf::from(&site.path);
    let public = root.join("public");
    let private = root.join("private");

    site.status = SiteStatus::Provisioning;
    registry::update_site(app, &site)?;

    let result = (|| {
        clear_directory(&public)?;
        copy_tree(&wordpress, &public)?;

        let maria = if site.database_mode == DatabaseMode::Mariadb {
            Some(processes::ensure_site_database(app, state, &site)?)
        } else {
            None
        };

        if site.database_mode == DatabaseMode::Sqlite {
            let plugin = runtime_dir.join("plugins").join("sqlite-database-integration");
            for required in [plugin.join("db.copy"), plugin.join("load.php")] {
                if !required.exists() {
                    return Err(format!("SQLite integration is missing: {}", required.display()));
                }
            }

            let destination = public.join("wp-content").join("plugins").join("sqlite-database-integration");
            copy_tree(&plugin, &destination)?;

            let mut dropin = fs::read_to_string(destination.join("db.copy"))
                .map_err(|error| format!("Unable to read SQLite db.copy: {error}"))?;
            dropin = dropin
                .replace(
                    "'{SQLITE_IMPLEMENTATION_FOLDER_PATH}'",
                    "__DIR__.'/plugins/sqlite-database-integration'",
                )
                .replace("{SQLITE_PLUGIN}", "sqlite-database-integration/load.php");
            fs::write(public.join("wp-content").join("db.php"), dropin)
                .map_err(|error| format!("Unable to write SQLite db.php: {error}"))?;
        }

        let sqlite_path = if site.database_mode == DatabaseMode::Sqlite {
            Some(private.join("database.sqlite"))
        } else {
            None
        };

        write_config(
            &public,
            site.database_mode.clone(),
            sqlite_path.as_deref(),
            maria.as_ref(),
        )?;

        let admin_user = "admin".to_string();
        let admin_password = format!(
            "PinWP-{}-{}!",
            &Uuid::new_v4().simple().to_string()[..8],
            &Uuid::new_v4().simple().to_string()[..8]
        );
        let site_url = format!("http://{}:{}", site.domain, site.port);

        run_wp_version(
            app,
            &public,
            &[
                "core".into(), "install".into(), format!("--url={site_url}"),
                format!("--title={}", site.name), format!("--admin_user={admin_user}"),
                format!("--admin_password={admin_password}"),
                "--admin_email=admin@pinwp.local".into(), "--skip-email".into(),
            ],
            &site.php_version,
        )?;

        if site.database_mode == DatabaseMode::Sqlite {
            run_wp_version(
                app,
                &public,
                &[
                    "plugin".into(), "activate".into(),
                    "sqlite-database-integration/load.php".into(),
                ],
                &site.php_version,
            )?;
        }

        run_wp_version(
            app,
            &public,
            &["rewrite".into(), "structure".into(), "/%postname%/".into()],
            &site.php_version,
        )?;

        Ok::<_, String>((admin_user, admin_password, site_url))
    })();

    match result {
        Ok((admin_user, admin_password, site_url)) => {
            site.provisioned = true;
            site.status = SiteStatus::Stopped;
            registry::update_site(app, &site)?;
            Ok(ProvisionResult { site, site_url, admin_user, admin_password })
        }
        Err(error) => {
            site.status = SiteStatus::Error;
            let _ = registry::update_site(app, &site);
            Err(error)
        }
    }
}
