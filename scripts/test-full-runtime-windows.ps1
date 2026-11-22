param([string]$RuntimeRoot="dist/runtime-windows-x64")
$ErrorActionPreference="Stop"
$repoRoot=Split-Path -Parent $PSScriptRoot
$runtime=Join-Path $repoRoot $RuntimeRoot
$php=Join-Path $runtime "php/php.exe"
$ini=Join-Path $runtime "php/php.ini"
$wp=Join-Path $runtime "wp-cli/wp-cli.phar"
$maria=Join-Path $runtime "mariadb/bin"
$temp=Join-Path ([IO.Path]::GetTempPath()) ("pinwp-full-"+[guid]::NewGuid().ToString("N"))
$data=Join-Path $temp "data"
$site=Join-Path $temp "public"
$port=33127
$password="PinWPFullRuntime!2026"
$proc=$null
try {
  New-Item -ItemType Directory -Force -Path $temp,$site | Out-Null
  & (Join-Path $maria "mariadb-install-db.exe") "--datadir=$data" "--password=$password" "--port=$port"
  if($LASTEXITCODE-ne 0){throw "MariaDB initialization failed"}

  $serverOut=Join-Path $temp "mariadb-out.log"
  $serverErr=Join-Path $temp "mariadb-err.log"
  $defaults=Join-Path $data "my.ini"
  if(-not(Test-Path $defaults)){throw "MariaDB bootstrap did not generate my.ini"}
  $proc=Start-Process (Join-Path $maria "mariadbd.exe") -ArgumentList @("--defaults-file=$defaults","--port=$port","--bind-address=127.0.0.1","--skip-name-resolve","--console") -RedirectStandardOutput $serverOut -RedirectStandardError $serverErr -PassThru -WindowStyle Hidden
  $ready=$false
  for($i=0;$i-lt 60;$i++){
    Start-Sleep -Milliseconds 250
    $client = New-Object System.Net.Sockets.TcpClient
    try { $client.Connect("127.0.0.1",$port); $ready=$true; break } catch {} finally { $client.Dispose() }
    if($proc.HasExited){break}
  }
  if(-not $ready){
    Write-Host "=== MariaDB stdout ==="; if(Test-Path $serverOut){Get-Content $serverOut}
    Write-Host "=== MariaDB stderr ==="; if(Test-Path $serverErr){Get-Content $serverErr}
    throw "MariaDB did not become ready"
  }
  & (Join-Path $maria "mariadb-admin.exe") --protocol=tcp -h127.0.0.1 "-P$port" -uroot "-p$password" ping
  if($LASTEXITCODE-ne 0){throw "MariaDB root authentication failed"}

  & (Join-Path $maria "mariadb.exe") -h127.0.0.1 "-P$port" -uroot "-p$password" -e "CREATE DATABASE pinwp_full CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci; CREATE USER 'pinwp'@'127.0.0.1' IDENTIFIED BY 'pinwp'; GRANT ALL PRIVILEGES ON pinwp_full.* TO 'pinwp'@'127.0.0.1'; FLUSH PRIVILEGES;"
  if($LASTEXITCODE-ne 0){throw "MariaDB site database creation failed"}

  Copy-Item (Join-Path $runtime "wordpress/*") $site -Recurse -Force
  & $php -c $ini $wp "--path=$site" config create --dbname=pinwp_full --dbuser=pinwp --dbpass=pinwp "--dbhost=127.0.0.1:$port" --skip-check --force
  if($LASTEXITCODE-ne 0){throw "wp config create failed"}
  & $php -c $ini $wp "--path=$site" core install --url=http://127.0.0.1:9192 --title="PinWP Full Runtime Test" --admin_user=admin --admin_password=pinwp-full-test --admin_email=admin@pinwp.local --skip-email
  if($LASTEXITCODE-ne 0){throw "Full-mode WordPress installation failed"}

  $url=(& $php -c $ini $wp "--path=$site" option get siteurl).Trim()
  if($url-ne"http://127.0.0.1:9192"){throw "Unexpected Full mode siteurl: $url"}

  & (Join-Path $runtime "cloudflared/cloudflared.exe") --version
  & (Join-Path $runtime "caddy/caddy.exe") version
  & (Join-Path $runtime "mailpit/mailpit.exe") version
  if(-not(Test-Path(Join-Path $runtime "connector/pinwp-connector.zip"))){throw "Connector package missing"}

  Write-Host "PINWP FULL MODE SMOKE PASS"
}
finally {
  if($proc){Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue}
  if(Test-Path $temp){Remove-Item $temp -Recurse -Force}
}
