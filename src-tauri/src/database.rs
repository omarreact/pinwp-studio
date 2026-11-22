use crate::{models::{DatabaseMode,DatabaseResult},processes::{self,ProcessRegistry},registry,runtime};
use std::{fs,path::PathBuf,process::Command};
use tauri::{AppHandle,State};

fn is_read_only(sql:&str)->bool{let t=sql.trim_start().to_ascii_lowercase();["select","show","describe","desc","pragma","explain","with"].iter().any(|p|t.starts_with(p))}
pub fn query(app:&AppHandle,state:&State<'_,ProcessRegistry>,site_id:&str,sql:&str,allow_write:bool)->Result<DatabaseResult,String>{
    if sql.trim().is_empty(){return Err("SQL is empty.".into())}
    if !allow_write&&!is_read_only(sql){return Err("Write SQL is blocked. Enable write mode explicitly for this query.".into())}
    let site=registry::get_site(app,site_id)?;if !site.provisioned{return Err("WordPress is not installed.".into())}
    let output=match site.database_mode{
        DatabaseMode::Mariadb=>processes::mariadb_query(app,state,&site,sql)?,
        DatabaseMode::Sqlite=>{
            let script=PathBuf::from(&site.path).join(".pinwp").join("db-query.php");
            fs::write(&script,r#"<?php
$db=getenv('PINWP_DB');$sql=getenv('PINWP_SQL');$pdo=new PDO('sqlite:'.$db);$pdo->setAttribute(PDO::ATTR_ERRMODE,PDO::ERRMODE_EXCEPTION);
$stmt=$pdo->prepare($sql);$stmt->execute();if($stmt->columnCount()>0){echo json_encode($stmt->fetchAll(PDO::FETCH_ASSOC),JSON_PRETTY_PRINT|JSON_UNESCAPED_SLASHES);}else{echo "Rows affected: ".$stmt->rowCount();}
"#).map_err(|e|e.to_string())?;
            let db=PathBuf::from(&site.path).join("private").join("database.sqlite");
            let out=Command::new(runtime::php_exe(app)?).arg("-c").arg(runtime::php_ini(app)?).arg(&script).env("PINWP_DB",&db).env("PINWP_SQL",sql).output().map_err(|e|e.to_string())?;
            if !out.status.success(){return Err(String::from_utf8_lossy(&out.stderr).to_string())}
            String::from_utf8_lossy(&out.stdout).to_string()
        }
    };
    Ok(DatabaseResult{output,rows_affected:None})
}
