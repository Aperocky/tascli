#!/bin/sh
CONFIG_DIR="$HOME/.config/tascli"
CONFIG_PATH="$HOME/.config/tascli/config.json"
BACKUP_FILE="/tmp/tascli_config_backup.json"
TMP_DB_FILE="/tmp/tascli.db"

CREATED_DIR=0
if [ ! -d "$CONFIG_DIR" ]; then
    echo "No config directory detected, creating one"
    mkdir -p "$CONFIG_DIR"
    CREATED_DIR=1
elif [ -f "$CONFIG_PATH" ]; then
    echo "Backing up existing config file"
    cp "$CONFIG_PATH" "$BACKUP_FILE"
fi
echo '{"data_dir": "/tmp"}' >"$CONFIG_PATH"

# Original benchmarks
hyperfine -r 50 'tascli task -c benchmark "task performance benchmark"' -n "Task Insertion"
hyperfine -r 50 'tascli list task -c benchmark' -n "List Tasks"
hyperfine -r 50 'i=$((HYPERFINE_ITERATION % 50 + 1)); yes | tascli delete $i' -n "Task Deletion"

if [ -f "$BACKUP_FILE" ]; then
    cp "$BACKUP_FILE" "$CONFIG_PATH"
    rm "$BACKUP_FILE"
    echo "Restored original config"
elif [ "$CREATED_DIR" -eq 1 ]; then
    rm "$CONFIG_PATH"
    rmdir "$CONFIG_DIR"
    echo "Cleaned up config file created"
fi
rm "$TMP_DB_FILE"
echo "Cleaned up test db"
