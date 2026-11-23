<?php
/**
 * Plugin Name: PinWP Connector
 * Description: Authenticated health and file-deployment bridge for PinWP Studio.
 * Version: 1.0.0
 */
if (!defined('ABSPATH')) exit;

add_action('rest_api_init', function () {
    $permission = function () { return current_user_can('manage_options'); };

    register_rest_route('pinwp/v1', '/health', [
        'methods' => 'GET',
        'permission_callback' => $permission,
        'callback' => function () {
            return [
                'ok' => true,
                'wordpress' => get_bloginfo('version'),
                'site' => home_url('/'),
                'php' => PHP_VERSION,
            ];
        },
    ]);

    register_rest_route('pinwp/v1', '/deploy', [
        'methods' => 'POST',
        'permission_callback' => $permission,
        'callback' => function () {
            if (empty($_FILES['package']['tmp_name'])) {
                return new WP_Error('pinwp_missing_package', 'Deployment package is required.', ['status' => 400]);
            }
            if (!class_exists('ZipArchive')) {
                return new WP_Error('pinwp_zip_missing', 'ZipArchive is required.', ['status' => 500]);
            }

            $upload = wp_upload_dir();
            // Store complete WordPress backups outside the web document root.
            // If the hosting layout cannot guarantee this, refuse deployment.
            $root = realpath(ABSPATH);
            $document_root = realpath(isset($_SERVER['DOCUMENT_ROOT']) ? $_SERVER['DOCUMENT_ROOT'] : ABSPATH);
            if (!$root || !$document_root) {
                return new WP_Error('pinwp_unsafe_backup', 'Unable to verify the private backup location.', ['status' => 500]);
            }
            $backup_dir = dirname($root) . DIRECTORY_SEPARATOR . '.pinwp-private-backups';
            if (!wp_mkdir_p($backup_dir)) {
                return new WP_Error('pinwp_backup_failed', 'Unable to create a private backup directory.', ['status' => 500]);
            }
            $backup_dir = realpath($backup_dir);
            if (!$backup_dir || $backup_dir === $document_root ||
                strpos($backup_dir . DIRECTORY_SEPARATOR, trailingslashit($document_root)) === 0) {
                return new WP_Error('pinwp_unsafe_backup', 'Backups would be web-accessible. Deployment refused.', ['status' => 403]);
            }
            @chmod($backup_dir, 0700);
            $stamp = gmdate('Ymd-His');
            $backup = $backup_dir . '/pre-pinwp-' . $stamp . '.zip';

            $zip = new ZipArchive();
            if ($zip->open($backup, ZipArchive::CREATE | ZipArchive::OVERWRITE) !== true) {
                return new WP_Error('pinwp_backup_failed', 'Unable to create backup archive.', ['status' => 500]);
            }
            $iterator = new RecursiveIteratorIterator(
                new RecursiveDirectoryIterator($root, FilesystemIterator::SKIP_DOTS),
                RecursiveIteratorIterator::LEAVES_ONLY
            );
            foreach ($iterator as $file) {
                if (!$file->isFile()) continue;
                $path = $file->getRealPath();
                if (strpos($path, $backup_dir) === 0) continue;
                $zip->addFile($path, ltrim(str_replace($root, '', $path), DIRECTORY_SEPARATOR));
            }
            $zip->close();

            $temp = trailingslashit($upload['basedir']) . 'pinwp-deploy-' . wp_generate_uuid4();
            wp_mkdir_p($temp);
            $package = new ZipArchive();
            if ($package->open($_FILES['package']['tmp_name']) !== true) {
                return new WP_Error('pinwp_invalid_package', 'Unable to open PinWP package.', ['status' => 400]);
            }
            // Validate archive entry paths before extraction (Zip Slip prevention).
            for ($i = 0; $i < $package->numFiles; $i++) {
                $name = $package->getNameIndex($i);
                $normalized = str_replace('\\', '/', $name);
                $segments = explode('/', $normalized);
                if ($name === false || strpos($normalized, '/') === 0 ||
                    preg_match('/^[A-Za-z]:/', $normalized) ||
                    in_array('..', $segments, true) || strpos($normalized, "\0") !== false) {
                    $package->close();
                    return new WP_Error('pinwp_unsafe_archive', 'Unsafe archive entry detected.', ['status' => 400]);
                }
            }
            if (!$package->extractTo($temp)) {
                $package->close();
                return new WP_Error('pinwp_extract_failed', 'Unable to extract deployment package.', ['status' => 500]);
            }
            $package->close();

            $source = trailingslashit($temp) . 'public';
            if (!is_dir($source)) {
                return new WP_Error('pinwp_invalid_layout', 'PinWP package does not contain public/.', ['status' => 400]);
            }

            $copy = function($src, $dst) use (&$copy) {
                if (!is_dir($dst)) wp_mkdir_p($dst);
                foreach (scandir($src) as $name) {
                    if ($name === '.' || $name === '..') continue;
                    $s = $src . DIRECTORY_SEPARATOR . $name;
                    $d = $dst . DIRECTORY_SEPARATOR . $name;
                    $relative = ltrim(str_replace(ABSPATH, '', $d), DIRECTORY_SEPARATOR);
                    $normalized = str_replace('\\', '/', $relative);
                    if ($relative === 'wp-config.php' || $normalized === 'wp-content/db.php') continue;
                    if (is_dir($s)) $copy($s, $d); else copy($s, $d);
                }
            };
            $copy($source, ABSPATH);

            return [
                'ok' => true,
                'backup' => $backup,
                'message' => 'Files deployed. Database changes are intentionally not auto-applied by Connector v1.',
            ];
        },
    ]);
});
