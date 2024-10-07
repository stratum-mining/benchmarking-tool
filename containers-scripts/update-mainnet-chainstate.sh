#!/bin/sh

# Input parameter for network (used for future expansion)
NETWORK=$1

# If NETWORK is not mainnet (""), exit the script
if [ "$NETWORK" != "" ]; then
    echo "Script should only run when NETWORK is mainnet. Exiting."
    exit 0
fi

# Shared volume path for snapshot storage
SNAPSHOT_DIR="/shared_volume"
# Paths for chainstate directory in each container
CHAINSTATE_DIR="/root/.bitcoin/chainstate"
# Path for the extraction completion flag
EXTRACTION_COMPLETION_FLAG="/root/.bitcoin/extraction_completion.flag"
# Path for the integrity flag
INTEGRITY_FLAG="$SNAPSHOT_DIR/integrity.flag"
# Backup URL and interval settings
BACKUP_BASE_URL="http://75.119.150.111/backup"
DOWNLOAD_INTERVAL_DAYS=1  # 1 day = 24 hours
# Maximum retries for downloading
MAX_RETRIES=30
RETRY_INTERVAL=60  # 1 minute retry interval
# Lock settings
LOCK_FILE="$SNAPSHOT_DIR/download.lock"

# Function to clean up the lock file
cleanup() {
    rm -f "$LOCK_FILE"
}

# Trap to ensure the lock file is removed on exit
trap cleanup EXIT

# Function to download the snapshot and verify it
download_snapshot() {
    echo "Downloading snapshot..."

    retry_count=0

    # Remove old backup files
    echo "Removing old backup files from the shared volume..."
    rm -f "$SNAPSHOT_DIR"/backup_mainnet_blocks_chainstate_*.tar.gz
    rm -f "$SNAPSHOT_DIR"/backup_mainnet_blocks_chainstate_*.tar.gz.sha256
    echo "false" > "$INTEGRITY_FLAG"

    while [ $retry_count -lt $MAX_RETRIES ]; do
        echo "Attempt $((retry_count + 1)) of $MAX_RETRIES to download the snapshot..."

        BACKUP_FILE_NAME="backup_mainnet_blocks_chainstate_$(date -u +"%Y-%m-%d_%H-UTC").tar.gz"
        BACKUP_HASH_FILE_NAME="$BACKUP_FILE_NAME.sha256"
        BACKUP_URL="$BACKUP_BASE_URL/$BACKUP_FILE_NAME"
        BACKUP_HASH_URL="$BACKUP_URL.sha256"
        BACKUP_FILE="$SNAPSHOT_DIR/$BACKUP_FILE_NAME"
        BACKUP_HASH_FILE="$SNAPSHOT_DIR/$BACKUP_HASH_FILE_NAME"

        if wget "$BACKUP_URL" -O "$BACKUP_FILE" && wget "$BACKUP_HASH_URL" -O "$BACKUP_HASH_FILE"; then
            echo "Download succeeded. Verifying the snapshot hash..."

            cd "$SNAPSHOT_DIR" || exit
            if sha256sum -c "$BACKUP_HASH_FILE"; then
                # Set the integrity flag and release the lock immediately after integrity check
                echo "true" > "$INTEGRITY_FLAG"
                echo "Snapshot integrity verified. Releasing lock for other containers to proceed with extraction."
                return 0  # Successful download and verification
            else
                echo "Hash verification failed! Retrying..."
                rm -f "$BACKUP_FILE" "$BACKUP_HASH_FILE"
            fi
        else
            echo "Download failed! Retrying in $RETRY_INTERVAL seconds..."
        fi

        retry_count=$((retry_count + 1))
        sleep $RETRY_INTERVAL
    done

    echo "Failed to download the snapshot after $MAX_RETRIES attempts. Aborting."
    return 1  # Unsuccessful download
}

# Function to handle extraction
extract_snapshot() {
    echo "Cleaning up the chainstate directory..."
    rm -rf "$CHAINSTATE_DIR"/*
    echo "$LATEST_BACKUP_FILE"
    echo "Extracting the downloaded snapshot..."
    if tar -xzvf "$LATEST_BACKUP_FILE" -C /root/.bitcoin; then
        # Set the extraction completion flag to true after successful extraction
        echo "true" > "$EXTRACTION_COMPLETION_FLAG"
        echo "Extraction completed successfully."
    else
        # Set the extraction completion flag to false if extraction fails
        echo "false" > "$EXTRACTION_COMPLETION_FLAG"
        echo "Extraction failed!"
        exit 1  # Exit the script with error code
    fi
}

# Check if the local chainstate directory is updated and integrity verified for this container
if [ -d "$CHAINSTATE_DIR" ]; then
    CHAINSTATE_MOD_TIME=$(stat -c %Y "$CHAINSTATE_DIR")
    CURRENT_TIME=$(date +%s)
    TIME_DIFF=$(( (CURRENT_TIME - CHAINSTATE_MOD_TIME) / 86400 ))

    if [ "$TIME_DIFF" -lt "$DOWNLOAD_INTERVAL_DAYS" ] && [ -f "$EXTRACTION_COMPLETION_FLAG" ]; then
        FLAG_VALUE=$(cat "$EXTRACTION_COMPLETION_FLAG" | tr -d '\n')
        if [ "$FLAG_VALUE" = "true" ]; then
            echo "Container chainstate is updated and extraction is complete. Exiting."
            exit 0
        else
            echo "EXTRACTION_COMPLETION_FLAG is not set to true. Proceeding with further checks."
        fi
    fi

else
    echo "No local chainstate found. Proceeding with snapshot download check."
fi

# Check if a recent and verified snapshot already exists
LATEST_BACKUP_FILE=$(find "$SNAPSHOT_DIR" -maxdepth 1 -name "backup_mainnet_blocks_chainstate_*.tar.gz" -type f -printf "%T@ %p\n" | sort -n | tail -1 | cut -d' ' -f2)

if [ -f "$LATEST_BACKUP_FILE" ]; then
    BACKUP_MOD_TIME=$(stat -c %Y "$LATEST_BACKUP_FILE")
    TIME_DIFF=$(( (CURRENT_TIME - BACKUP_MOD_TIME) / 86400 ))

    if [ "$TIME_DIFF" -lt "$DOWNLOAD_INTERVAL_DAYS" ] && [ -f "$INTEGRITY_FLAG" ] && grep -q "true" "$INTEGRITY_FLAG"; then
        echo "Recent snapshot with verified integrity found. Proceeding to extraction."

        # Set the extraction completion flag to false before extraction
        echo "false" > "$EXTRACTION_COMPLETION_FLAG"
        extract_snapshot
        exit 0
    else
        echo "No recent or verified snapshot found. Proceeding with download."
    fi
fi

# Use flock for the lock file to prevent concurrent downloads
{
    flock -n 9 || {
        echo "Another container is currently downloading the snapshot. Waiting for download to finish..."
        while [ -f "$LOCK_FILE" ]; do
            sleep 5  # Wait for 5 seconds before checking again
        done
        echo "Download finished. Checking integrity..."
        LATEST_BACKUP_FILE=$(find "$SNAPSHOT_DIR" -maxdepth 1 -name "backup_mainnet_blocks_chainstate_*.tar.gz" -type f -printf "%T@ %p\n" | sort -n | tail -1 | cut -d' ' -f2)
        if [ -f "$INTEGRITY_FLAG" ] && grep -q "true" "$INTEGRITY_FLAG"; then
            echo "Snapshot integrity verified. Proceeding to extraction."
            extract_snapshot
            exit 0
        else
            echo "Integrity flag not set or snapshot corrupted. Exiting."
            exit 1
        fi
    }

    echo "Lock acquired. Proceeding with snapshot download..."
    touch "$LOCK_FILE"  # Create the lock file

    download_snapshot || exit 1  # Exit if download fails
    # Here, we release the lock so other containers can download or extract if needed
    cleanup
} 9>"$LOCK_FILE"  # Using file descriptor 9 for the lock file

LATEST_BACKUP_FILE=$(find "$SNAPSHOT_DIR" -maxdepth 1 -name "backup_mainnet_blocks_chainstate_*.tar.gz" -type f -printf "%T@ %p\n" | sort -n | tail -1 | cut -d' ' -f2)
extract_snapshot

echo "Update and extraction completed successfully."
