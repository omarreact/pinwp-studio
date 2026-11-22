param([string]$OutputRoot = "dist/runtime-windows-x64")
$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$sources = Get-Content (Join-Path $repoRoot "runtime/sources.json") -Raw | ConvertFrom-Json
$temp = Join-Path ([System.IO.Path]::GetTempPath()) ("pinwp-runtime-" + [guid]::NewGuid().ToString("N"))

function Download-File([string]$Url,[string]$Destination) {
  Write-Host "Downloading $Url"
  Invoke-WebRequest -Uri $Url -OutFile $Destination -UseBasicParsing
  if (-not (Test-Path $Destination)) { throw "Download failed: $Url" }
}
function File-Sha256([string]$Path) { (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
function Verify-Sha([string]$Path,[string]$Expected,[string]$Name) {
  $actual = File-Sha256 $Path
  if ($actual -ne $Expected.ToLowerInvariant()) { throw "$Name SHA256 mismatch. Expected $Expected, received $actual" }
}

try {
  New-Item -ItemType Directory -Force -Path $temp | Out-Null
  $output = Join-Path $repoRoot $OutputRoot
  if (Test-Path $output) { Remove-Item $output -Recurse -Force }
  New-Item -ItemType Directory -Force -Path $output | Out-Null

  $phpZip=Join-Path $temp "php.zip"
  $wpZip=Join-Path $temp "wordpress.zip"
  $sqliteZip=Join-Path $temp "sqlite.zip"
  $wpCli=Join-Path $temp "wp-cli.phar"
  $mariaZip=Join-Path $temp "mariadb.zip"
  $cloud=Join-Path $temp "cloudflared.exe"
  $caddyZip=Join-Path $temp "caddy.zip"
  $mailpitZip=Join-Path $temp "mailpit.zip"

  Download-File $sources.php.url $phpZip
  Download-File $sources.wordpress.url $wpZip
  Download-File $sources.sqliteIntegration.url $sqliteZip
  Download-File $sources.wpCli.url $wpCli
  Download-File $sources.mariadb.url $mariaZip
  Download-File $sources.cloudflared.url $cloud
  Download-File $sources.caddy.url $caddyZip
  Download-File $sources.mailpit.url $mailpitZip

  Verify-Sha $wpCli $sources.wpCli.sha256 "WP-CLI"
  Verify-Sha $mariaZip $sources.mariadb.sha256 "MariaDB"
  Verify-Sha $cloud $sources.cloudflared.sha256 "cloudflared"
  Verify-Sha $caddyZip $sources.caddy.sha256 "Caddy"
  Verify-Sha $mailpitZip $sources.mailpit.sha256 "Mailpit"

  $phpDir=Join-Path $output "php"
  New-Item -ItemType Directory -Force -Path $phpDir | Out-Null
  Expand-Archive $phpZip $phpDir -Force
@'
[PHP]
extension_dir="ext"
date.timezone=UTC
memory_limit=256M
max_execution_time=120
upload_max_filesize=128M
post_max_size=128M
display_errors=On
display_startup_errors=On
error_reporting=E_ALL
log_errors=On
SMTP=127.0.0.1
smtp_port=1025
sendmail_from=wordpress@pinwp.local
extension=curl
extension=gd
extension=intl
extension=mbstring
extension=openssl
extension=pdo_sqlite
extension=sqlite3
extension=mysqli
extension=pdo_mysql
extension=zip
'@ | Set-Content (Join-Path $phpDir "php.ini") -Encoding UTF8

  $wpExtract=Join-Path $temp "wp"
  Expand-Archive $wpZip $wpExtract -Force
  Copy-Item (Join-Path $wpExtract "wordpress") (Join-Path $output "wordpress") -Recurse

  $sqliteExtract=Join-Path $temp "sqlite"
  Expand-Archive $sqliteZip $sqliteExtract -Force
  $sqliteSource=Get-ChildItem $sqliteExtract -Directory | Where-Object { Test-Path (Join-Path $_.FullName "db.copy") } | Select-Object -First 1
  if (-not $sqliteSource) { throw "SQLite archive layout not recognized" }
  $plugins=Join-Path $output "plugins"
  New-Item -ItemType Directory -Force -Path $plugins | Out-Null
  Copy-Item $sqliteSource.FullName (Join-Path $plugins "sqlite-database-integration") -Recurse

  $wpCliDir=Join-Path $output "wp-cli"
  New-Item -ItemType Directory -Force -Path $wpCliDir | Out-Null
  Copy-Item $wpCli (Join-Path $wpCliDir "wp-cli.phar")

  $mariaExtract=Join-Path $temp "maria"
  Expand-Archive $mariaZip $mariaExtract -Force
  $mariaSource=Get-ChildItem $mariaExtract -Directory | Select-Object -First 1
  if (-not $mariaSource) { throw "MariaDB archive layout not recognized" }
  Copy-Item $mariaSource.FullName (Join-Path $output "mariadb") -Recurse

  $cloudDir=Join-Path $output "cloudflared"
  New-Item -ItemType Directory -Force -Path $cloudDir | Out-Null
  Copy-Item $cloud (Join-Path $cloudDir "cloudflared.exe")

  $caddyDir=Join-Path $output "caddy"
  New-Item -ItemType Directory -Force -Path $caddyDir | Out-Null
  Expand-Archive $caddyZip $caddyDir -Force

  $mailpitDir=Join-Path $output "mailpit"
  New-Item -ItemType Directory -Force -Path $mailpitDir | Out-Null
  Expand-Archive $mailpitZip $mailpitDir -Force

  Copy-Item (Join-Path $repoRoot "runtime/version-catalog.json") (Join-Path $output "version-catalog.json")
  $connectorDir=Join-Path $output "connector"
  New-Item -ItemType Directory -Force -Path $connectorDir | Out-Null
  $connectorStage=Join-Path $temp "connector"
  New-Item -ItemType Directory -Force -Path (Join-Path $connectorStage "pinwp-connector") | Out-Null
  Copy-Item (Join-Path $repoRoot "connector/pinwp-connector.php") (Join-Path $connectorStage "pinwp-connector/pinwp-connector.php")
  Compress-Archive -Path (Join-Path $connectorStage "pinwp-connector") -DestinationPath (Join-Path $connectorDir "pinwp-connector.zip") -Force

  $phpExe=Join-Path $phpDir "php.exe"
  $extensions=& $phpExe -c (Join-Path $phpDir "php.ini") -m
  foreach($required in @("PDO","pdo_sqlite","sqlite3","mbstring","openssl","mysqli","pdo_mysql")) {
    if($extensions -notcontains $required){throw "Required PHP extension '$required' is not enabled."}
  }

  & $phpExe -c (Join-Path $phpDir "php.ini") (Join-Path $wpCliDir "wp-cli.phar") --info
  if($LASTEXITCODE -ne 0){throw "WP-CLI check failed"}
  & (Join-Path $output "mariadb/bin/mariadbd.exe") --version
  if($LASTEXITCODE -ne 0){throw "MariaDB check failed"}
  & (Join-Path $cloudDir "cloudflared.exe") --version
  if($LASTEXITCODE -ne 0){throw "cloudflared check failed"}
  & (Join-Path $caddyDir "caddy.exe") version
  if($LASTEXITCODE -ne 0){throw "Caddy check failed"}
  & (Join-Path $mailpitDir "mailpit.exe") version
  if($LASTEXITCODE -ne 0){throw "Mailpit check failed"}

  $manifest=[ordered]@{
    runtimeVersion=$sources.runtimeVersion
    platform=$sources.platform
    builtAt=(Get-Date).ToUniversalTime().ToString("o")
    php=[ordered]@{version=$sources.php.version;sha256=File-Sha256 $phpZip}
    wordpress=[ordered]@{version=$sources.wordpress.version;sha256=File-Sha256 $wpZip}
    sqliteIntegration=[ordered]@{version=$sources.sqliteIntegration.version;sha256=File-Sha256 $sqliteZip}
    wpCli=[ordered]@{version=$sources.wpCli.version;sha256=File-Sha256 $wpCli}
    mariadb=[ordered]@{version=$sources.mariadb.version;sha256=File-Sha256 $mariaZip}
    cloudflared=[ordered]@{version=$sources.cloudflared.version;sha256=File-Sha256 $cloud}
    caddy=[ordered]@{version=$sources.caddy.version;sha256=File-Sha256 $caddyZip}
    mailpit=[ordered]@{version=$sources.mailpit.version;sha256=File-Sha256 $mailpitZip}
  }
  $manifest | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $output "runtime.json") -Encoding UTF8
  Write-Host "PINWP RUNTIME V1 BUILD PASS"
}
finally {
  if(Test-Path $temp){Remove-Item $temp -Recurse -Force}
}
