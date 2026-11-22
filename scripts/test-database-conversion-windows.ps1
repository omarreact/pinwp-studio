param([string]$RuntimeRoot="dist/runtime-windows-x64")
$ErrorActionPreference="Stop"
$repoRoot=Split-Path -Parent $PSScriptRoot
$runtime=Join-Path $repoRoot $RuntimeRoot
$php=Join-Path $runtime "php/php.exe"
$ini=Join-Path $runtime "php/php.ini"
$wp=Join-Path $runtime "wp-cli/wp-cli.phar"
$maria=Join-Path $runtime "mariadb/bin"
$helper=Join-Path $runtime "helpers/sqlite-to-mysql.php"
$temp=Join-Path ([IO.Path]::GetTempPath()) ("pinwp-convert-"+[guid]::NewGuid().ToString("N"))
$sqliteSite=Join-Path $temp "sqlite-public"
$mariaSite=Join-Path $temp "maria-public"
$private=Join-Path $temp "private"
$data=Join-Path $temp "maria-data"
$sql=Join-Path $temp "database.sql"
$port=33128
$rootPassword="PinWPConversion!2026"
$proc=$null

try {
  New-Item -ItemType Directory -Force -Path $temp,$sqliteSite,$mariaSite,$private | Out-Null
  Copy-Item (Join-Path $runtime "wordpress/*") $sqliteSite -Recurse -Force
  $pluginDestination=Join-Path $sqliteSite "wp-content/plugins/sqlite-database-integration"
  New-Item -ItemType Directory -Force -Path (Split-Path $pluginDestination -Parent) | Out-Null
  Copy-Item (Join-Path $runtime "plugins/sqlite-database-integration") $pluginDestination -Recurse -Force
  $dropin=Get-Content (Join-Path $pluginDestination "db.copy") -Raw
  $dropin=$dropin.Replace("'{SQLITE_IMPLEMENTATION_FOLDER_PATH}'","__DIR__.'/plugins/sqlite-database-integration'")
  $dropin=$dropin.Replace("{SQLITE_PLUGIN}","sqlite-database-integration/load.php")
  Set-Content (Join-Path $sqliteSite "wp-content/db.php") $dropin -Encoding UTF8

  & $php -c $ini $wp "--path=$sqliteSite" config create --dbname=pinwp --dbuser=pinwp --dbpass=pinwp --dbhost=localhost --skip-check --skip-salts --force
  if($LASTEXITCODE-ne 0){throw "SQLite wp-config creation failed"}
  $dbPath=(Join-Path $private "database.sqlite").Replace("\","/")
  $configPath=Join-Path $sqliteSite "wp-config.php"
  $config=Get-Content $configPath -Raw
  $anchor="/* That's all, stop editing!"
  $config=$config.Replace($anchor,"define( 'DB_PATH', '$dbPath' );"+[Environment]::NewLine+$anchor)
  Set-Content $configPath $config -Encoding UTF8

  & $php -c $ini $wp "--path=$sqliteSite" core install --url=http://source.localhost:9193 --title="PinWP Conversion Test" --admin_user=admin --admin_password=conversion-test --admin_email=admin@pinwp.local --skip-email
  if($LASTEXITCODE-ne 0){throw "SQLite WordPress install failed"}
  & $php -c $ini $wp "--path=$sqliteSite" option update pinwp_conversion_test "serialized-safe-value"
  & $php -c $ini $wp "--path=$sqliteSite" post create --post_type=post --post_status=publish --post_title="Conversion Post" --porcelain
  if($LASTEXITCODE-ne 0){throw "SQLite fixture creation failed"}

  $env:PINWP_DB=$dbPath
  & $php -c $ini $helper | Set-Content $sql -Encoding UTF8
  Remove-Item Env:\PINWP_DB
  if($LASTEXITCODE-ne 0 -or -not(Test-Path $sql)){throw "SQLite to MySQL export failed"}

  & (Join-Path $maria "mariadb-install-db.exe") "--datadir=$data" "--password=$rootPassword" "--port=$port"
  if($LASTEXITCODE-ne 0){throw "MariaDB conversion target initialization failed"}
  $defaults=Join-Path $data "my.ini"
  $serverOut=Join-Path $temp "mariadb-out.log"
  $serverErr=Join-Path $temp "mariadb-err.log"
  $proc=Start-Process (Join-Path $maria "mariadbd.exe") -ArgumentList @("--defaults-file=$defaults","--port=$port","--bind-address=127.0.0.1","--skip-name-resolve","--console") -RedirectStandardOutput $serverOut -RedirectStandardError $serverErr -PassThru -WindowStyle Hidden
  $ready=$false
  for($i=0;$i-lt 60;$i++){
    Start-Sleep -Milliseconds 250
    $client=New-Object System.Net.Sockets.TcpClient
    try{$client.Connect("127.0.0.1",$port);$ready=$true;break}catch{}finally{$client.Dispose()}
    if($proc.HasExited){break}
  }
  if(-not $ready){if(Test-Path $serverErr){Get-Content $serverErr};throw "MariaDB conversion target did not become ready"}

  & (Join-Path $maria "mariadb.exe") "--host=127.0.0.1" "--port=$port" "--user=root" "--password=$rootPassword" -e "CREATE DATABASE pinwp_convert CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci; CREATE USER 'pinwp'@'127.0.0.1' IDENTIFIED BY 'pinwp'; GRANT ALL PRIVILEGES ON pinwp_convert.* TO 'pinwp'@'127.0.0.1'; FLUSH PRIVILEGES;"
  if($LASTEXITCODE-ne 0){throw "MariaDB conversion database creation failed"}
  Get-Content $sql -Raw | & (Join-Path $maria "mariadb.exe") "--host=127.0.0.1" "--port=$port" "--user=pinwp" "--password=pinwp" pinwp_convert
  if($LASTEXITCODE-ne 0){throw "Converted SQL import failed"}

  Copy-Item (Join-Path $runtime "wordpress/*") $mariaSite -Recurse -Force
  & $php -c $ini $wp "--path=$mariaSite" config create --dbname=pinwp_convert --dbuser=pinwp --dbpass=pinwp "--dbhost=127.0.0.1:$port" --skip-check --force
  if($LASTEXITCODE-ne 0){throw "MariaDB target wp-config failed"}

  $value=(& $php -c $ini $wp "--path=$mariaSite" option get pinwp_conversion_test).Trim()
  if($value-ne"serialized-safe-value"){throw "Converted WordPress option did not survive: $value"}
  $count=(& $php -c $ini $wp "--path=$mariaSite" post list --post_type=post --format=count).Trim()
  if([int]$count-lt 1){throw "Converted WordPress posts were not readable"}

  Write-Host "PINWP SQLITE TO MYSQL CONVERSION PASS"
}
finally {
  Remove-Item Env:\PINWP_DB -ErrorAction SilentlyContinue
  if($proc){Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue}
  if(Test-Path $temp){Remove-Item $temp -Recurse -Force}
}
