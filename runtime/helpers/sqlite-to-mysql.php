<?php
declare(strict_types=1);
$db = getenv('PINWP_DB');
if (!$db || !is_file($db)) { fwrite(STDERR, "PINWP_DB is missing or not a file.\n"); exit(2); }
$pdo = new PDO('sqlite:' . $db);
$pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);
$pdo->setAttribute(PDO::ATTR_DEFAULT_FETCH_MODE, PDO::FETCH_ASSOC);

function qi(string $name): string { return chr(96) . str_replace(chr(96), chr(96) . chr(96), $name) . chr(96); }
function mysql_type(string $sqliteType): string {
    $type = strtoupper(trim($sqliteType));
    $unsigned = str_contains($type, 'UNSIGNED') ? ' UNSIGNED' : '';
    if (preg_match('/BIGINT(?:\(\d+\))?/i', $type)) return 'BIGINT' . $unsigned;
    if (preg_match('/MEDIUMINT(?:\(\d+\))?/i', $type)) return 'MEDIUMINT' . $unsigned;
    if (preg_match('/SMALLINT(?:\(\d+\))?/i', $type)) return 'SMALLINT' . $unsigned;
    if (preg_match('/TINYINT(?:\(\d+\))?/i', $type)) return 'TINYINT' . $unsigned;
    if (preg_match('/(?:^|\s)INT(?:EGER)?(?:\(\d+\))?/i', $type)) return 'BIGINT' . $unsigned;
    if (preg_match('/VARCHAR\((\d+)\)/i', $type, $m)) return 'VARCHAR(' . $m[1] . ')';
    if (preg_match('/CHAR\((\d+)\)/i', $type, $m)) return 'CHAR(' . $m[1] . ')';
    if (str_contains($type, 'LONGTEXT')) return 'LONGTEXT';
    if (str_contains($type, 'MEDIUMTEXT')) return 'MEDIUMTEXT';
    if (str_contains($type, 'TINYTEXT')) return 'TINYTEXT';
    if (str_contains($type, 'TEXT') || str_contains($type, 'CLOB')) return 'TEXT';
    if (str_contains($type, 'LONGBLOB')) return 'LONGBLOB';
    if (str_contains($type, 'MEDIUMBLOB')) return 'MEDIUMBLOB';
    if (str_contains($type, 'BLOB')) return 'BLOB';
    if (preg_match('/DECIMAL\((\d+),\s*(\d+)\)/i', $type, $m)) return 'DECIMAL(' . $m[1] . ',' . $m[2] . ')';
    if (str_contains($type, 'DOUBLE') || str_contains($type, 'FLOAT') || str_contains($type, 'REAL')) return 'DOUBLE';
    if (str_contains($type, 'DATETIME')) return 'DATETIME';
    if (str_contains($type, 'TIMESTAMP')) return 'TIMESTAMP';
    if (str_contains($type, 'DATE')) return 'DATE';
    if (str_contains($type, 'TIME')) return 'TIME';
    return 'LONGTEXT';
}
/**
 * SQLite stores indexed strings as TEXT; MariaDB requires bounded index prefixes
 * under utf8mb4 (4 bytes per character). Keep column values intact.
 */
function mysql_index_component(string $name, string $sqliteType, int $componentCount): string {
    $mysqlType = mysql_type($sqliteType);
    $prefix = max(1, min(191, intdiv(768, max(1, $componentCount))));
    $needsPrefix = str_contains($mysqlType, 'TEXT') || str_contains($mysqlType, 'BLOB');
    if (preg_match('/^(?:VAR)?CHAR\\((\\d+)\\)/i', $mysqlType, $match)) {
        $needsPrefix = (int)$match[1] > $prefix;
    }
    return qi($name) . ($needsPrefix ? '(' . $prefix . ')' : '');
}

function value_sql(mixed $value): string {
    if ($value === null) return 'NULL';
    if (is_int($value) || is_float($value)) return (string)$value;
    $value = (string)$value;
    if (!mb_check_encoding($value, 'UTF-8') || str_contains($value, "\0")) return "X'" . bin2hex($value) . "'";
    return "'" . str_replace(["\\", "'"], ["\\\\", "''"], $value) . "'";
}

echo "SET NAMES utf8mb4;\nSET FOREIGN_KEY_CHECKS=0;\nSET SQL_MODE='NO_AUTO_VALUE_ON_ZERO';\n\n";
$tables = $pdo->query("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")->fetchAll(PDO::FETCH_COLUMN);
// Tables prefixed _wp_sqlite_ belong to the SQLite compatibility driver, not WordPress.
// They are meaningless in MariaDB and may use SQLite-only TEXT primary keys.
$tables = array_values(array_filter($tables, static function (string $table): bool {
    return !str_starts_with($table, '_wp_sqlite_');
}));

foreach ($tables as $table) {
    $columns = $pdo->query("PRAGMA table_info(" . qi($table) . ")")->fetchAll();
    if (!$columns) continue;
    $primary = array_values(array_filter($columns, fn($column) => (int)$column['pk'] > 0));
    usort($primary, fn($a, $b) => (int)$a['pk'] <=> (int)$b['pk']);
    $singleAuto = count($primary) === 1 && preg_match('/INT/i', (string)$primary[0]['type']);
    $definitions = [];

    foreach ($columns as $column) {
        $definition = qi((string)$column['name']) . ' ' . mysql_type((string)$column['type']);
        if ((int)$column['notnull'] === 1 || (int)$column['pk'] > 0) $definition .= ' NOT NULL';
        if ($column['dflt_value'] !== null) $definition .= ' DEFAULT ' . $column['dflt_value'];
        if ($singleAuto && (int)$column['pk'] > 0) $definition .= ' AUTO_INCREMENT';
        $definitions[] = $definition;
    }
    if ($primary) {
        $definitions[] = 'PRIMARY KEY (' . implode(',', array_map(
            fn($column) => mysql_index_component((string)$column['name'], (string)$column['type'], count($primary)),
            $primary
        )) . ')';
    }

    $indexes = $pdo->query("PRAGMA index_list(" . qi($table) . ")")->fetchAll();
    foreach ($indexes as $index) {
        $name = (string)$index['name'];
        if (str_starts_with($name, 'sqlite_autoindex_') || ($index['origin'] ?? '') === 'pk') continue;
        $info = $pdo->query("PRAGMA index_info(" . qi($name) . ")")->fetchAll();
        if (!$info) continue;
        usort($info, fn($a, $b) => (int)$a['seqno'] <=> (int)$b['seqno']);
        $columnMap = array_column($columns, null, 'name');
        $cols = implode(',', array_map(static function ($row) use ($columnMap, $info): string {
            $name = (string) ($row['name'] ?? '');
            if ($name === '' || !isset($columnMap[$name])) {
                throw new RuntimeException('Cannot migrate unsupported SQL expression index.');
            }
            return mysql_index_component($name, (string)$columnMap[$name]['type'], count($info));
        }, $info));
        $definitions[] = ((int)$index['unique'] === 1 ? 'UNIQUE KEY ' : 'KEY ') . qi($name) . ' (' . $cols . ')';
    }

    echo 'DROP TABLE IF EXISTS ' . qi($table) . ";\n";
    echo 'CREATE TABLE ' . qi($table) . " (\n  " . implode(",\n  ", $definitions) . "\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;\n";

    $statement = $pdo->query('SELECT * FROM ' . qi($table));
    while ($row = $statement->fetch()) {
        $names = implode(',', array_map('qi', array_keys($row)));
        $values = implode(',', array_map('value_sql', array_values($row)));
        echo 'INSERT INTO ' . qi($table) . " ($names) VALUES ($values);\n";
    }
    echo "\n";
}
echo "SET FOREIGN_KEY_CHECKS=1;\n";
