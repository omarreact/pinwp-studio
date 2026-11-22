<?php
/**
 * Plugin Name: PinWP Connector
 * Description: Authenticated, backup-first file deployment bridge for PinWP Studio.
 * Version: 1.1.0
 * Requires PHP: 8.0
 */
if (!defined('ABSPATH')) {
    exit;
}

/**
 * Resolve persistent private storage. Neither the WordPress directory nor the
 * current web document root may contain backup archives or extracted packages.
 */
function pinwp_private_directory() {
    $wp_root = realpath(ABSPATH);
    $document_root = !empty($_SERVER['DOCUMENT_ROOT']) ? realpath($_SERVER['DOCUMENT_ROOT']) : $wp_root;
    $candidates = [];
    if (defined('PINWP_BACKUP_DIR') && PINWP_BACKUP_DIR) {
        $candidates[] = PINWP_BACKUP_DIR;
    }
    $candidates[] = dirname($wp_root) . DIRECTORY_SEPARATOR . '.pinwp-backups';
    $candidates[] = sys_get_temp_dir() . DIRECTORY_SEPARATOR .
        'pinwp-private-' . substr(hash('sha256', $wp_root . wp_salt('auth')), 0, 20);

    foreach ($candidates as $candidate) {
        if (!is_dir($candidate) && !@mkdir($candidate, 0700, true)) {
            continue;
        }
        $directory = realpath($candidate);
        if (!$directory || !is_writable($directory)) {
            continue;
        }
        $private = rtrim($directory, DIRECTORY_SEPARATOR) . DIRECTORY_SEPARATOR;
        $safe = true;
        foreach ([$wp_root, $document_root] as $public) {
            if (!$public) {
                continue;
            }
            $public = rtrim($public, DIRECTORY_SEPARATOR);
            if ($directory === $public || str_starts_with($private, $public . DIRECTORY_SEPARATOR)) {
                $safe = false;
                break;
            }
        }
        if ($safe) {
            @chmod($directory, 0700);
            return $directory;
        }
    }
    throw new RuntimeException(
        'No private backup directory is available. Configure PINWP_BACKUP_DIR outside the web root.'
    );
}

function pinwp_error($code, $message, $status = 400) {
    return new WP_Error($code, $message, ['status' => $status]);
}

/** Reject zip-slip, absolute paths, symlinks and unusually large archive expansion. */
function pinwp_archive_entries($zip, $only_public) {
    if ($zip->numFiles > 50000) {
        throw new RuntimeException('Package contains too many files.');
    }
    $total = 0;
    $entries = [];
    for ($i = 0; $i < $zip->numFiles; $i++) {
        $name = $zip->getNameIndex($i);
        if (!is_string($name) || strlen($name) > 2048 || str_contains($name, "\\") ||
            str_contains($name, "\0") || str_starts_with($name, '/') ||
            preg_match('/^[A-Za-z]:/', $name)) {
            throw new RuntimeException('Unsafe archive entry path.');
        }
        $is_directory = str_ends_with($name, '/');
        $parts = explode('/', $is_directory ? substr($name, 0, -1) : $name);
        if (!$parts || array_filter($parts, static function ($piece) {
            return $piece === '' || $piece === '.' || $piece === '..';
        })) {
            throw new RuntimeException('Archive contains a traversal or invalid entry.');
        }
        $stat = $zip->statIndex($i);
        if (!$stat) {
            throw new RuntimeException('Unable to examine an archive entry.');
        }
        if ($zip->getExternalAttributesIndex($i, $os, $attributes) &&
            $os === ZipArchive::OPSYS_UNIX &&
            (($attributes >> 16) & 0170000) === 0120000) {
            throw new RuntimeException('Archive contains an unsupported symbolic link.');
        }
        $total += $stat['size'];
        if ($total > 2 * 1024 * 1024 * 1024) {
            throw new RuntimeException('Expanded package exceeds the 2 GiB safety limit.');
        }
        if ($only_public && !str_starts_with($name, 'public/') &&
            !in_array($name, ['manifest.json', 'database.sql', 'database.sqlite'], true)) {
            throw new RuntimeException('Unexpected file outside the public package folder.');
        }
        $entries[] = [$name, $is_directory];
    }
    return $entries;
}

/** Extract validated paths by streaming, never by blindly calling extractTo(). */
function pinwp_extract_archive($archive, $destination, $only_public) {
    $zip = new ZipArchive();
    if ($zip->open($archive) !== true) {
        throw new RuntimeException('Cannot open ZIP archive.');
    }
    try {
        $entries = pinwp_archive_entries($zip, $only_public);
        foreach ($entries as [$name, $directory]) {
            $path = $destination . DIRECTORY_SEPARATOR . str_replace('/', DIRECTORY_SEPARATOR, $name);
            if ($directory) {
                if (!is_dir($path) && !wp_mkdir_p($path)) {
                    throw new RuntimeException('Unable to create staging directory.');
                }
                continue;
            }
            if (!wp_mkdir_p(dirname($path))) {
                throw new RuntimeException('Unable to create staging directory.');
            }
            $in = $zip->getStream($name);
            $out = @fopen($path, 'wb');
            if (!$in || !$out) {
                if (is_resource($in)) fclose($in);
                if (is_resource($out)) fclose($out);
                throw new RuntimeException('Unable to unpack an archive entry.');
            }
            $copied = stream_copy_to_stream($in, $out);
            fclose($in);
            fclose($out);
            if ($copied === false) {
                throw new RuntimeException('Archive extraction failed.');
            }
        }
    } finally {
        $zip->close();
    }
}

function pinwp_remove_directory($directory) {
    if (!is_dir($directory) || is_link($directory)) {
        return;
    }
    foreach (new FilesystemIterator($directory) as $entry) {
        if ($entry->isDir() && !$entry->isLink()) {
            pinwp_remove_directory($entry->getPathname());
        } else {
            @unlink($entry->getPathname());
        }
    }
    @rmdir($directory);
}

/** Copy staged files without following destination symlinks or replacing local DB configuration. */
function pinwp_copy_files($source, $destination, $preserve_config) {
    if (!is_dir($source)) {
        throw new RuntimeException('The selected archive contains no files to restore.');
    }
    $root = realpath($destination);
    if (!$root) {
        throw new RuntimeException('Invalid target site root.');
    }
    $iterator = new RecursiveIteratorIterator(
        new RecursiveDirectoryIterator($source, FilesystemIterator::SKIP_DOTS),
        RecursiveIteratorIterator::LEAVES_ONLY
    );
    foreach ($iterator as $file) {
        if (!$file->isFile() || $file->isLink()) {
            throw new RuntimeException('Unsupported file type in the package.');
        }
        $relative = substr($file->getPathname(), strlen($source) + 1);
        $normalized = str_replace(DIRECTORY_SEPARATOR, '/', $relative);
        if ($preserve_config && in_array($normalized, ['wp-config.php', 'wp-content/db.php'], true)) {
            continue;
        }
        if (str_starts_with($normalized, '.git/') || str_starts_with($normalized, '.pinwp/')) {
            continue;
        }
        $target = $root . DIRECTORY_SEPARATOR . $relative;
        $parent = dirname($target);
        if (!wp_mkdir_p($parent) || !is_dir($parent)) {
            throw new RuntimeException('Unable to create target directory.');
        }
        $parent_real = realpath($parent);
        if (!$parent_real || ($parent_real !== $root &&
            !str_starts_with($parent_real, $root . DIRECTORY_SEPARATOR)) ||
            is_link($target) || is_dir($target)) {
            throw new RuntimeException('Unsafe destination file path.');
        }
        if (!@copy($file->getPathname(), $target)) {
            throw new RuntimeException('Unable to write a deployed file.');
        }
    }
}

function pinwp_backup_files($directory) {
    $root = realpath(ABSPATH);
    $name = 'pre-pinwp-' . gmdate('Ymd-His') . '-' . substr(wp_generate_uuid4(), 0, 8) . '.zip';
    $path = $directory . DIRECTORY_SEPARATOR . $name;
    $zip = new ZipArchive();
    if ($zip->open($path, ZipArchive::CREATE | ZipArchive::EXCL) !== true) {
        throw new RuntimeException('Unable to create the pre-deployment backup.');
    }
    try {
        $files = new RecursiveIteratorIterator(
            new RecursiveDirectoryIterator($root, FilesystemIterator::SKIP_DOTS),
            RecursiveIteratorIterator::LEAVES_ONLY
        );
        foreach ($files as $file) {
            if ($file->isLink()) continue;
            if (!$file->isFile()) continue;
            $real = $file->getRealPath();
            if (!$real || !str_starts_with($real, $root . DIRECTORY_SEPARATOR)) continue;
            $relative = substr($real, strlen($root) + 1);
            if (!$zip->addFile($real, str_replace(DIRECTORY_SEPARATOR, '/', $relative))) {
                throw new RuntimeException('Unable to include a file in the backup.');
            }
        }
    } finally {
        if (!$zip->close()) {
            throw new RuntimeException('Unable to finalize the backup archive.');
        }
    }
    return $path;
}

add_action('rest_api_init', function () {
    $can_manage = static function () { return current_user_can('manage_options'); };

    register_rest_route('pinwp/v1', '/health', [
        'methods' => 'GET',
        'permission_callback' => $can_manage,
        'callback' => static function () {
            return [
                'ok' => true,
                'wordpress' => get_bloginfo('version'),
                'site' => home_url('/'),
                'php' => PHP_VERSION,
                'connector' => '1.1.0',
            ];
        },
    ]);

    register_rest_route('pinwp/v1', '/deploy', [
        'methods' => 'POST',
        'permission_callback' => $can_manage,
        'callback' => static function () {
            if (!class_exists('ZipArchive')) {
                return pinwp_error('pinwp_zip_missing', 'ZipArchive extension is required.', 500);
            }
            if (empty($_FILES['package']['tmp_name']) ||
                (int) ($_FILES['package']['error'] ?? UPLOAD_ERR_NO_FILE) !== UPLOAD_ERR_OK) {
                return pinwp_error('pinwp_missing_package', 'Valid deployment package is required.');
            }

            $staging = null;
            $lock = null;
            try {
                $private = pinwp_private_directory();
                $lock = fopen($private . DIRECTORY_SEPARATOR . 'deployment.lock', 'c');
                if (!$lock || !flock($lock, LOCK_EX | LOCK_NB)) {
                    return pinwp_error('pinwp_busy', 'Another deployment is running.', 409);
                }
                $staging = $private . DIRECTORY_SEPARATOR . 'stage-' . wp_generate_uuid4();
                if (!wp_mkdir_p($staging)) {
                    throw new RuntimeException('Unable to create private deployment staging.');
                }
                pinwp_extract_archive($_FILES['package']['tmp_name'], $staging, true);
                if (!is_dir($staging . DIRECTORY_SEPARATOR . 'public')) {
                    throw new RuntimeException('Package must contain a public/ directory.');
                }
                $backup = pinwp_backup_files($private);
                // Publish the recovery pointer BEFORE changing live files, even if copy fails mid-way.
                $written = file_put_contents($private . DIRECTORY_SEPARATOR . 'latest.json',
                    wp_json_encode(['archive' => basename($backup), 'date' => gmdate('c')]), LOCK_EX);
                if ($written === false) {
                    throw new RuntimeException('Backup created, but its recovery record could not be written.');
                }
                pinwp_copy_files($staging . DIRECTORY_SEPARATOR . 'public', ABSPATH, true);
                return [
                    'ok' => true,
                    'backup' => 'Private pre-deployment file backup created',
                    'rollback_available' => true,
                    'message' => 'Files deployed. Database migration is not automated by Connector v1.',
                ];
            } catch (Throwable $error) {
                return pinwp_error('pinwp_deploy_failed', $error->getMessage(), 500);
            } finally {
                if ($staging) pinwp_remove_directory($staging);
                if ($lock) {
                    flock($lock, LOCK_UN);
                    fclose($lock);
                }
            }
        },
    ]);

    register_rest_route('pinwp/v1', '/rollback', [
        'methods' => 'POST',
        'permission_callback' => $can_manage,
        'callback' => static function () {
            $stage = null;
            $lock = null;
            try {
                $private = pinwp_private_directory();
                $lock = fopen($private . DIRECTORY_SEPARATOR . 'deployment.lock', 'c');
                if (!$lock || !flock($lock, LOCK_EX | LOCK_NB)) {
                    return pinwp_error('pinwp_busy', 'Another deployment is running.', 409);
                }
                $metadata = $private . DIRECTORY_SEPARATOR . 'latest.json';
                if (!is_file($metadata)) {
                    return pinwp_error('pinwp_backup_missing', 'There is no previous deployment backup.', 404);
                }
                $state = json_decode(file_get_contents($metadata), true);
                $name = $state['archive'] ?? '';
                if (!is_string($name) || !preg_match('/^pre-pinwp-[0-9]{8}-[0-9]{6}-[a-f0-9-]+\.zip$/', $name)) {
                    throw new RuntimeException('Invalid backup record.');
                }
                $backup = $private . DIRECTORY_SEPARATOR . $name;
                if (!is_file($backup)) {
                    return pinwp_error('pinwp_backup_missing', 'Backup archive is unavailable.', 404);
                }
                $stage = $private . DIRECTORY_SEPARATOR . 'restore-' . wp_generate_uuid4();
                if (!wp_mkdir_p($stage)) {
                    throw new RuntimeException('Unable to create restore staging.');
                }
                pinwp_extract_archive($backup, $stage, false);
                pinwp_copy_files($stage, ABSPATH, false);
                return [
                    'ok' => true,
                    'message' => 'Backup files restored. Files added after the backup are retained. Database was not changed.',
                    'mode' => 'file-restore',
                ];
            } catch (Throwable $error) {
                return pinwp_error('pinwp_rollback_failed', $error->getMessage(), 500);
            } finally {
                if ($stage) pinwp_remove_directory($stage);
                if ($lock) {
                    flock($lock, LOCK_UN);
                    fclose($lock);
                }
            }
        },
    ]);
});
