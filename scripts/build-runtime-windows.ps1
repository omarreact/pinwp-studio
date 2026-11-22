param(
  [string]$OutputRoot = "dist/runtime-windows-x64"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$sources = Get-Content (Join-Path $repoRoot "runtime/sources.json") -Raw | ConvertFrom-Json
$temp = Join-Path ([System.IO.Path]::GetTempPath()) ("pinwp-runtime-" + [guid]::NewGuid().ToString("N"))

function Download-File([string]$Url, [string]$Destination) {
  Write-Host "Downloading $Url"
  Invoke-WebRequest -Uri $Url -OutFile $Destination -UseBasicParsing
  if (-not (Test-Path $Destination)) { throw "Download failed: $Url" }
}

function File-Sha256([string]$Path) {
  (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

try {
  New-Item -ItemType Directory -Force -Path $temp | Out-Null
  $output = Join-Path $repoRoot $OutputRoot
  if (Test-Path $output) { Remove-Item $output -Recurse -Force }
  New-Item -ItemType Directory -Force -Path $output | Out-Null

  $phpZip = Join-Path $temp "php.zip"
  $wordpressZip = Join-Path $temp "wordpress.zip"
  $sqliteZip = Join-Path $temp "sqlite.zip"
  $wpCliPhar = Join-Path $temp "wp-cli.phar"

  Download-File $sources.php.url $phpZip
  Download-File $sources.wordpress.url $wordpressZip
  Download-File $sources.sqliteIntegration.url $sqliteZip
  Download-File $sources.wpCli.url $wpCliPhar

  $wpCliHash = File-Sha256 $wpCliPhar
  if ($wpCliHash -ne $sources.wpCli.sha256.ToLowerInvariant()) {
    throw "WP-CLI SHA256 mismatch. Expected $($sources.wpCli.sha256), received $wpCliHash"
  }

  $phpDir = Join-Path $output "php"
  New-Item -ItemType Directory -Force -Path $phpDir | Out-Null
  Expand-Archive -Path $phpZip -DestinationPath $phpDir -Force

  $phpIni = @'
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
'@
  Set-Content -Path (Join-Path $phpDir "php.ini") -Value $phpIni -Encoding UTF8

  $wordpressExtract = Join-Path $temp "wp-extract"
  Expand-Archive -Path $wordpressZip -DestinationPath $wordpressExtract -Force
  $wordpressSource = Join-Path $wordpressExtract "wordpress"
  if (-not (Test-Path (Join-Path $wordpressSource "wp-settings.php"))) {
    throw "WordPress archive layout is not recognized."
  }
  Copy-Item $wordpressSource (Join-Path $output "wordpress") -Recurse

  $sqliteExtract = Join-Path $temp "sqlite-extract"
  Expand-Archive -Path $sqliteZip -DestinationPath $sqliteExtract -Force
  $sqliteSource = Get-ChildItem $sqliteExtract -Directory | Where-Object {
    Test-Path (Join-Path $_.FullName "db.copy")
  } | Select-Object -First 1
  if (-not $sqliteSource) { throw "SQLite integration archive does not contain db.copy." }
  $pluginsDir = Join-Path $output "plugins"
  New-Item -ItemType Directory -Force -Path $pluginsDir | Out-Null
  Copy-Item $sqliteSource.FullName (Join-Path $pluginsDir "sqlite-database-integration") -Recurse

  $wpCliDir = Join-Path $output "wp-cli"
  New-Item -ItemType Directory -Force -Path $wpCliDir | Out-Null
  Copy-Item $wpCliPhar (Join-Path $wpCliDir "wp-cli.phar")

  $phpExe = Join-Path $phpDir "php.exe"
  if (-not (Test-Path $phpExe)) { throw "PHP executable was not found after extraction." }

  $extensions = & $phpExe -c (Join-Path $phpDir "php.ini") -m
  foreach ($required in @("PDO", "pdo_sqlite", "sqlite3", "mbstring", "openssl")) {
    if ($extensions -notcontains $required) { throw "Required PHP extension '$required' is not enabled." }
  }

  $wpCliInfo = & $phpExe -c (Join-Path $phpDir "php.ini") (Join-Path $wpCliDir "wp-cli.phar") --info
  if ($LASTEXITCODE -ne 0) { throw "WP-CLI runtime check failed." }

  $manifest = [ordered]@{
    runtimeVersion = $sources.runtimeVersion
    platform = $sources.platform
    builtAt = (Get-Date).ToUniversalTime().ToString("o")
    php = [ordered]@{ version = $sources.php.version; sha256 = (File-Sha256 $phpZip) }
    wordpress = [ordered]@{ version = $sources.wordpress.version; sha256 = (File-Sha256 $wordpressZip) }
    sqliteIntegration = [ordered]@{ version = $sources.sqliteIntegration.version; sha256 = (File-Sha256 $sqliteZip) }
    wpCli = [ordered]@{ version = $sources.wpCli.version; sha256 = $wpCliHash }
  }
  $manifest | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $output "runtime.json") -Encoding UTF8

  Write-Host "Runtime pack built: $output"
  Write-Host ($wpCliInfo -join [Environment]::NewLine)
}
finally {
  if (Test-Path $temp) { Remove-Item $temp -Recurse -Force }
}
