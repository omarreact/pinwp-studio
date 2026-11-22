use crate::{
    models::{CompatibilityMatrixReport, CompatibilityMatrixRow, DatabaseMode},
    processes::{self, ProcessRegistry},
    provision, registry, runtime, versions,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn install_sqlite_dropin(app: &AppHandle, public: &Path) -> Result<(), String> {
    let plugin = runtime::resolve_runtime_dir(app)?
        .join("plugins")
        .join("sqlite-database-integration");
    let destination = public
        .join("wp-content")
        .join("plugins")
        .join("sqlite-database-integration");
    copy_tree(&plugin, &destination)?;

    let mut dropin = fs::read_to_string(destination.join("db.copy"))
        .map_err(|error| error.to_string())?;
    dropin = dropin
        .replace(
            "'{SQLITE_IMPLEMENTATION_FOLDER_PATH}'",
            "__DIR__.'/plugins/sqlite-database-integration'",
        )
        .replace("{SQLITE_PLUGIN}", "sqlite-database-integration/load.php");
    fs::write(public.join("wp-content").join("db.php"), dropin)
        .map_err(|error| error.to_string())
}

fn create_config(
    app: &AppHandle,
    public: &Path,
    private: &Path,
    php_version: &str,
) -> Result<(), String> {
    provision::run_wp_version(
        app,
        public,
        &[
            "config".into(),
            "create".into(),
            "--dbname=pinwp_matrix".into(),
            "--dbuser=pinwp".into(),
            "--dbpass=pinwp".into(),
            "--dbhost=localhost".into(),
            "--skip-check".into(),
            "--skip-salts".into(),
            "--force".into(),
        ],
        php_version,
    )?;

    let path = public.join("wp-config.php");
    let mut config = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let anchor = "/* That's all, stop editing!";
    if !config.contains(anchor) {
        return Err("Matrix wp-config.php insertion anchor is missing.".into());
    }
    let db = private.join("database.sqlite").to_string_lossy().replace('\\', "/");
    config = config.replacen(
        anchor,
        &format!(
            "define( 'DB_PATH', '{}' );\ndefine( 'WP_ENVIRONMENT_TYPE', 'local' );\n{}",
            db.replace('\'', "\\'"),
            anchor
        ),
        1,
    );
    fs::write(path, config).map_err(|error| error.to_string())
}

fn lines(value: Result<String, String>) -> Vec<String> {
    value
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn run(
    app: &AppHandle,
    state: &State<'_, ProcessRegistry>,
    site_id: &str,
) -> Result<CompatibilityMatrixReport, String> {
    let site = registry::get_site(app, site_id)?;
    if !site.provisioned {
        return Err("Install WordPress before running the compatibility matrix.".into());
    }
    if site.database_mode == DatabaseMode::Mariadb {
        processes::ensure_mariadb(app, state)?;
    }

    let source_public = PathBuf::from(&site.path).join("public");
    let active_plugins = lines(provision::run_wp_version(
        app,
        &source_public,
        &[
            "plugin".into(),
            "list".into(),
            "--status=active".into(),
            "--field=name".into(),
        ],
        &site.php_version,
    ));
    let active_theme = lines(provision::run_wp_version(
        app,
        &source_public,
        &[
            "theme".into(),
            "list".into(),
            "--status=active".into(),
            "--field=name".into(),
        ],
        &site.php_version,
    ))
    .into_iter()
    .next();

    let catalog = versions::list(app)?;
    let php_versions: Vec<_> = catalog.php.into_iter().filter(|item| item.installed).collect();
    let wordpress_versions: Vec<_> = catalog
        .wordpress
        .into_iter()
        .filter(|item| item.installed)
        .collect();

    if php_versions.is_empty() || wordpress_versions.is_empty() {
        return Err("No installed runtime versions are available for the matrix.".into());
    }

    let matrix_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("matrix")
        .join(Uuid::new_v4().simple().to_string());
    fs::create_dir_all(&matrix_root).map_err(|error| error.to_string())?;

    let mut rows = Vec::new();

    for php in php_versions {
        for wordpress in &wordpress_versions {
            let case = matrix_root.join(format!("php-{}-wp-{}", php.key, wordpress.key));
            let public = case.join("public");
            let private = case.join("private");
            fs::create_dir_all(&private).map_err(|error| error.to_string())?;

            let result = (|| {
                copy_tree(&runtime::wordpress_dir_for(app, &wordpress.key)?, &public)?;

                copy_tree(
                    &source_public.join("wp-content").join("plugins"),
                    &public.join("wp-content").join("plugins"),
                )?;
                copy_tree(
                    &source_public.join("wp-content").join("themes"),
                    &public.join("wp-content").join("themes"),
                )?;

                install_sqlite_dropin(app, &public)?;
                create_config(app, &public, &private, &php.key)?;

                provision::run_wp_version(
                    app,
                    &public,
                    &[
                        "core".into(),
                        "install".into(),
                        "--url=http://matrix.localhost".into(),
                        "--title=PinWP Matrix".into(),
                        "--admin_user=admin".into(),
                        "--admin_password=pinwp-matrix".into(),
                        "--admin_email=matrix@pinwp.local".into(),
                        "--skip-email".into(),
                    ],
                    &php.key,
                )?;

                let mut output = Vec::new();
                for plugin in &active_plugins {
                    if plugin == "sqlite-database-integration" {
                        continue;
                    }
                    match provision::run_wp_version(
                        app,
                        &public,
                        &["plugin".into(), "activate".into(), plugin.clone()],
                        &php.key,
                    ) {
                        Ok(value) => output.push(value),
                        Err(error) => return Err(format!("Plugin {plugin}: {error}")),
                    }
                }

                if let Some(theme) = active_theme.as_ref() {
                    provision::run_wp_version(
                        app,
                        &public,
                        &["theme".into(), "activate".into(), theme.clone()],
                        &php.key,
                    )
                    .map_err(|error| format!("Theme {theme}: {error}"))?;
                }

                provision::run_wp_version(
                    app,
                    &public,
                    &["core".into(), "verify-checksums".into()],
                    &php.key,
                )?;
                let smoke = provision::run_wp_version(
                    app,
                    &public,
                    &[
                        "eval".into(),
                        "echo get_bloginfo('version') . ' PINWP_MATRIX_OK';".into(),
                    ],
                    &php.key,
                )?;
                output.push(smoke);
                Ok::<_, String>(output.join("\n"))
            })();

            match result {
                Ok(output) => rows.push(CompatibilityMatrixRow {
                    php_version: php.key.clone(),
                    wordpress_version: wordpress.key.clone(),
                    passed: true,
                    active_plugins_tested: active_plugins
                        .iter()
                        .filter(|plugin| plugin.as_str() != "sqlite-database-integration")
                        .count(),
                    active_theme: active_theme.clone(),
                    output,
                }),
                Err(error) => rows.push(CompatibilityMatrixRow {
                    php_version: php.key.clone(),
                    wordpress_version: wordpress.key.clone(),
                    passed: false,
                    active_plugins_tested: active_plugins.len(),
                    active_theme: active_theme.clone(),
                    output: error,
                }),
            }
        }
    }

    let _ = fs::remove_dir_all(&matrix_root);
    let passed = rows.iter().filter(|row| row.passed).count();
    let failed = rows.len().saturating_sub(passed);

    Ok(CompatibilityMatrixReport { rows, passed, failed })
}
