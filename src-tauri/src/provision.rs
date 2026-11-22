use crate::{models::{DatabaseMode, ProvisionResult, SiteStatus}, registry, runtime};
use std::{fs, path::{Path, PathBuf}, process::Command};
use tauri::AppHandle;
use uuid::Uuid;

fn copy_tree(source:&Path,destination:&Path)->Result<(),String>{
    fs::create_dir_all(destination).map_err(|e|format!("Unable to create {}: {e}",destination.display()))?;
    for entry in fs::read_dir(source).map_err(|e|format!("Unable to read {}: {e}",source.display()))?{
        let entry=entry.map_err(|e|format!("Unable to read runtime entry: {e}"))?;
        let source_path=entry.path();let destination_path=destination.join(entry.file_name());
        if source_path.is_dir(){copy_tree(&source_path,&destination_path)?}else{fs::copy(&source_path,&destination_path).map_err(|e|format!("Unable to copy {}: {e}",source_path.display()))?;}
    }
    Ok(())
}

fn clear_directory(path:&Path)->Result<(),String>{
    if !path.exists(){fs::create_dir_all(path).map_err(|e|format!("Unable to create {}: {e}",path.display()))?;return Ok(())}
    for entry in fs::read_dir(path).map_err(|e|format!("Unable to read {}: {e}",path.display()))?{
        let p=entry.map_err(|e|format!("Unable to read workspace entry: {e}"))?.path();
        if p.is_dir(){fs::remove_dir_all(&p)}else{fs::remove_file(&p)}.map_err(|e|format!("Unable to clean {}: {e}",p.display()))?;
    }
    Ok(())
}

fn php_escape(value:&str)->String{value.replace('\\',"/").replace('\'',"\\'")}

fn write_config(public:&Path,private:&Path)->Result<(),String>{
    let sample=public.join("wp-config-sample.php");
    let mut config=fs::read_to_string(&sample).map_err(|e|format!("Unable to read wp-config-sample.php: {e}"))?;
    config=config.replace("database_name_here","pinwp").replace("username_here","pinwp").replace("password_here","pinwp").replace("localhost","localhost");
    for key in ["AUTH_KEY","SECURE_AUTH_KEY","LOGGED_IN_KEY","NONCE_KEY","AUTH_SALT","SECURE_AUTH_SALT","LOGGED_IN_SALT","NONCE_SALT"]{
        let placeholder=format!("define( '{}', 'put your unique phrase here' );",key);
        let value=format!("{}{}",Uuid::new_v4(),Uuid::new_v4());
        config=config.replace(&placeholder,&format!("define( '{}', '{}' );",key,value));
    }
    config=config.replace("define( 'WP_DEBUG', false );","define( 'WP_DEBUG', true );\ndefine( 'WP_DEBUG_LOG', true );\ndefine( 'WP_DEBUG_DISPLAY', false );");
    let anchor="/* That's all, stop editing!";
    if !config.contains(anchor){return Err("Unable to locate wp-config.php insertion anchor.".into())}
    let db_path=php_escape(&private.join("database.sqlite").to_string_lossy());
    let local=format!("define( 'DB_PATH', '{}' );\ndefine( 'WP_ENVIRONMENT_TYPE', 'local' );\n",db_path);
    config=config.replacen(anchor,&format!("{}{}",local,anchor),1);
    fs::write(public.join("wp-config.php"),config).map_err(|e|format!("Unable to write wp-config.php: {e}"))
}

fn run_wp(app:&AppHandle,public:&Path,args:&[String])->Result<String,String>{
    let php=runtime::php_exe(app)?;let ini=runtime::php_ini(app)?;let wp_cli=runtime::wp_cli(app)?;
    let output=Command::new(php).arg("-c").arg(ini).arg(wp_cli).arg(format!("--path={}",public.to_string_lossy())).args(args).output().map_err(|e|format!("Unable to execute WP-CLI: {e}"))?;
    let stdout=String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr=String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success(){return Err(format!("WP-CLI failed: {}",if stderr.is_empty(){stdout}else{stderr}))}
    Ok(stdout)
}

pub fn provision(app:&AppHandle,id:&str)->Result<ProvisionResult,String>{
    let mut site=registry::get_site(app,id)?;
    if site.provisioned{return Err("This site is already provisioned.".into())}
    if site.database_mode!=DatabaseMode::Sqlite{return Err("MariaDB Full mode is not provisioned by Runtime Pack 0.1 yet. Create a Lite/SQLite site for this milestone.".into())}

    let runtime_dir=runtime::resolve_runtime_dir(app)?;
    let wordpress=runtime_dir.join("wordpress");
    let plugin=runtime_dir.join("plugins").join("sqlite-database-integration");
    for required in [wordpress.join("wp-settings.php"),plugin.join("db.copy"),plugin.join("load.php"),runtime::php_exe(app)?,runtime::wp_cli(app)?]{
        if !required.exists(){return Err(format!("Runtime Pack is incomplete: {}",required.display()))}
    }

    let root=PathBuf::from(&site.path);let public=root.join("public");let private=root.join("private");
    site.status=SiteStatus::Provisioning;registry::update_site(app,&site)?;

    let result=(||{
        clear_directory(&public)?;
        copy_tree(&wordpress,&public)?;
        let plugin_destination=public.join("wp-content").join("plugins").join("sqlite-database-integration");
        copy_tree(&plugin,&plugin_destination)?;
        let mut dropin=fs::read_to_string(plugin_destination.join("db.copy")).map_err(|e|format!("Unable to read SQLite db.copy: {e}"))?;
        dropin=dropin.replace("'{SQLITE_IMPLEMENTATION_FOLDER_PATH}'","__DIR__.'/plugins/sqlite-database-integration'").replace("{SQLITE_PLUGIN}","sqlite-database-integration/load.php");
        fs::write(public.join("wp-content").join("db.php"),dropin).map_err(|e|format!("Unable to write SQLite db.php: {e}"))?;
        write_config(&public,&private)?;

        let admin_user="admin".to_string();
        let admin_password=format!("PinWP-{}-{}!",&Uuid::new_v4().simple().to_string()[..8],&Uuid::new_v4().simple().to_string()[..8]);
        let site_url=format!("http://{}:{}",site.domain,site.port);
        run_wp(app,&public,&vec![
            "core".into(),"install".into(),format!("--url={site_url}"),format!("--title={}",site.name),
            format!("--admin_user={admin_user}"),format!("--admin_password={admin_password}"),
            "--admin_email=admin@pinwp.local".into(),"--skip-email".into()
        ])?;
        run_wp(app,&public,&vec!["plugin".into(),"activate".into(),"sqlite-database-integration/load.php".into()])?;
        run_wp(app,&public,&vec!["rewrite".into(),"structure".into(),"/%postname%/".into()])?;

        Ok::<_,String>((admin_user,admin_password,site_url))
    })();

    match result{
        Ok((admin_user,admin_password,site_url))=>{
            site.provisioned=true;site.status=SiteStatus::Stopped;registry::update_site(app,&site)?;
            Ok(ProvisionResult{site,site_url,admin_user,admin_password})
        }
        Err(error)=>{site.status=SiteStatus::Error;let _=registry::update_site(app,&site);Err(error)}
    }
}
