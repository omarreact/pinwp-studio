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
            $backup_dir = trailingslashit($upload['basedir']) . 'pinwp-backups';
            wp_mkdir_p($backup_dir);
            $stamp = gmdate('Ymd-His');
            $backup = $backup_dir . '/pre-pinwp-' . $stamp . '.zip';

            $zip = new ZipArchive();
            if ($zip->open($backup, ZipArchive::CREATE | ZipArchive::OVERWRITE) !== true) {
                return new WP_Error('pinwp_backup_failed', 'Unable to create backup archive.', ['status' => 500]);
            }
            $root = realpath(ABSPATH);
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
            $package->extractTo($temp);
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
