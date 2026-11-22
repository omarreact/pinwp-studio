param(
  [string]$RuntimeRoot = "dist/runtime-windows-x64"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$runtime = Join-Path $repoRoot $RuntimeRoot
$php = Join-Path $runtime "php/php.exe"
$phpIni = Join-Path $runtime "php/php.ini"
$wpCli = Join-Path $runtime "wp-cli/wp-cli.phar"
$wpTemplate = Join-Path $runtime "wordpress"
$sqlitePlugin = Join-Path $runtime "plugins/sqlite-database-integration"

foreach ($required in @($php, $phpIni, $wpCli, (Join-Path $wpTemplate "wp-settings.php"), (Join-Path $sqlitePlugin "db.copy"))) {
  if (-not (Test-Path $required)) { throw "Runtime smoke test missing: $required" }
}

$temp = Join-Path ([System.IO.Path]::GetTempPath()) ("pinwp-site-" + [guid]::NewGuid().ToString("N"))
$site = Join-Path $temp "public"
$private = Join-Path $temp "private"

try {
  New-Item -ItemType Directory -Force -Path $site, $private | Out-Null
  Copy-Item (Join-Path $wpTemplate "*") $site -Recurse -Force

  $pluginDestination = Join-Path $site "wp-content/plugins/sqlite-database-integration"
  New-Item -ItemType Directory -Force -Path (Split-Path $pluginDestination -Parent) | Out-Null
  Copy-Item $sqlitePlugin $pluginDestination -Recurse -Force

  $dbDropin = Get-Content (Join-Path $pluginDestination "db.copy") -Raw
  $dbDropin = $dbDropin.Replace("'{SQLITE_IMPLEMENTATION_FOLDER_PATH}'", "__DIR__.'/plugins/sqlite-database-integration'")
  $dbDropin = $dbDropin.Replace("{SQLITE_PLUGIN}", "sqlite-database-integration/load.php")
  Set-Content (Join-Path $site "wp-content/db.php") $dbDropin -Encoding UTF8

  & $php -c $phpIni $wpCli --path=$site config create --dbname=pinwp --dbuser=pinwp --dbpass=pinwp --dbhost=localhost --skip-check --skip-salts --force
  if ($LASTEXITCODE -ne 0) { throw "wp config create failed." }

  $dbPath = (Join-Path $private "database.sqlite").Replace("\", "/")
  $configPath = Join-Path $site "wp-config.php"
  $config = Get-Content $configPath -Raw
  $anchor = "/* That's all, stop editing!"
  if (-not $config.Contains($anchor)) { throw "wp-config.php insertion anchor was not found." }
  $dbLine = "define( 'DB_PATH', '$dbPath' );"
  $config = $config.Replace($anchor, $dbLine + [Environment]::NewLine + $anchor)
  Set-Content $configPath $config -Encoding UTF8

  & $php -c $phpIni $wpCli --path=$site core install --url="http://127.0.0.1:9191" --title="PinWP Runtime Test" --admin_user=admin --admin_password="pinwp-runtime-test-2026" --admin_email="admin@pinwp.local" --skip-email
  if ($LASTEXITCODE -ne 0) { throw "WordPress core install failed." }

  & $php -c $phpIni $wpCli --path=$site plugin activate sqlite-database-integration/load.php
  if ($LASTEXITCODE -ne 0) { throw "SQLite plugin activation failed." }

  $siteUrl = (& $php -c $phpIni $wpCli --path=$site option get siteurl).Trim()
  if ($siteUrl -ne "http://127.0.0.1:9191") { throw "Unexpected siteurl: $siteUrl" }
  if (-not (Test-Path $dbPath)) { throw "SQLite database was not created at protected DB_PATH: $dbPath" }

  $dbSize = (Get-Item $dbPath).Length
  if ($dbSize -lt 1024) { throw "SQLite database is unexpectedly small: $dbSize bytes" }

  Write-Host "PINWP RUNTIME SMOKE PASS"
  Write-Host "WordPress: $siteUrl"
  Write-Host "SQLite DB: $dbPath ($dbSize bytes)"
}
finally {
  if (Test-Path $temp) { Remove-Item $temp -Recurse -Force }
}
