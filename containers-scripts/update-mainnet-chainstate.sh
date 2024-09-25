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

# Paths for the timestamp file that marks when the snapshot was last downloaded
TIMESTAMP_FILE="$SNAPSHOT_DIR/last_download_timestamp"

# Backup URL and interval settings
BACKUP_BASE_URL="http://75.119.150.111/backup"
DOWNLOAD_INTERVAL_DAYS=1  # 1 day = 24 hours

# Maximum retries for downloading
MAX_RETRIES=30
RETRY_INTERVAL=60  # 1 minute retry interval

# Check if the local chainstate directory is updated for this container
if [ -d "$CHAINSTATE_DIR" ]; then
    CHAINSTATE_MOD_TIME=$(stat -c %Y "$CHAINSTATE_DIR")
    CURRENT_TIME=$(date +%s)
    TIME_DIFF=$(( (CURRENT_TIME - CHAINSTATE_MOD_TIME) / 86400 ))

    # Skip download if chainstate was updated recently
    if [ "$TIME_DIFF" -lt "$DOWNLOAD_INTERVAL_DAYS" ]; then
        echo "Container chainstate updated recently. Skipping download."
        exit 0
    fi
else
    echo "No local chainstate found. Proceeding with snapshot download check."
fi

# Shared download logic - checking the shared snapshot area
CURRENT_UTC=$(date -u +"%Y-%m-%d_%H-UTC")
BACKUP_FILE_NAME="backup_mainnet_blocks_chainstate_$CURRENT_UTC.tar.gz"
BACKUP_URL="$BACKUP_BASE_URL/$BACKUP_FILE_NAME"
BACKUP_HASH_URL="$BACKUP_URL.sha256"

BACKUP_FILE="$SNAPSHOT_DIR/$BACKUP_FILE_NAME"
BACKUP_HASH_FILE="$SNAPSHOT_DIR/$BACKUP_FILE_NAME.sha256"

# Create a lock file to prevent concurrent downloads
LOCK_FILE="$SNAPSHOT_DIR/download.lock"

# Function to clean up the lock file
cleanup() {
    rm -rf "$LOCK_FILE"
}

# Trap to ensure the lock file is removed on exit
trap cleanup EXIT

# Check if there is an existing backup file in the shared volume
LATEST_BACKUP_FILE=$(find /shared_volume -maxdepth 1 -name "backup_mainnet_blocks_chainstate_*.tar.gz" -type f -printf "%T@ %p\n" | sort -n | tail -1 | cut -d' ' -f2)

if [ -f "$TIMESTAMP_FILE" ]; then
    TIMESTAMP_MOD_TIME=$(stat -c %Y "$TIMESTAMP_FILE")
    TIME_DIFF=$(( (CURRENT_TIME - TIMESTAMP_MOD_TIME) / 86400 ))

    if [ "$TIME_DIFF" -lt "$DOWNLOAD_INTERVAL_DAYS" ]; then
        # Check if the latest backup file exists before extraction
        if [ -f "$LATEST_BACKUP_FILE" ]; then
            echo "Snapshot was downloaded recently. Proceeding to extraction for this container."
            tar -xzvf "$LATEST_BACKUP_FILE" -C /root/.bitcoin
            echo "Extraction complete."
            exit 0
        else
            echo "Warning: No backup file found. Unable to proceed with extraction."
            exit 1
        fi
    fi
fi

# Acquire the lock before downloading
if mkdir "$LOCK_FILE" 2>/dev/null; then
    echo "Lock acquired. Proceeding with snapshot download..."

    # Remove any old backup files in the shared volume
    echo "Removing old backup files from the shared volume..."
    rm -f "$SNAPSHOT_DIR/backup_mainnet_blocks_chainstate_*.tar.gz"
    rm -f "$SNAPSHOT_DIR/backup_mainnet_blocks_chainstate_*.tar.gz.sha256"

    # Retry downloading until successful or until maximum retries are reached
    retry_count=0
    success=0

    while [ $retry_count -lt $MAX_RETRIES ]; do
        echo "Attempt $((retry_count + 1)) of $MAX_RETRIES to download the snapshot..."

        if wget "$BACKUP_URL" -O "$BACKUP_FILE" && wget "$BACKUP_HASH_URL" -O "$BACKUP_HASH_FILE"; then
            echo "Download succeeded. Verifying the snapshot hash..."

            cd "$SNAPSHOT_DIR"
            if sha256sum -c "$BACKUP_HASH_FILE"; then
                echo "Hash verification succeeded."
                success=1
                break
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

    if [ $success -eq 0 ]; then
        echo "Failed to download the snapshot after $MAX_RETRIES attempts. Aborting."
        cleanup  # Clean up the lock file
        exit 1
    fi

    cleanup  # Clean up the lock file

    # Extract the snapshot to the container's local chainstate directory
    echo "Extracting the snapshot to the container's local chainstate..."
    tar -xzvf "$BACKUP_FILE" -C /root/.bitcoin

    # Update the timestamp file to mark the download time
    touch "$TIMESTAMP_FILE"

    echo "Update and extraction completed successfully."
else
    echo "Another container is currently downloading the snapshot. Waiting for it to complete..."

    # Wait for the download to finish and the lock to be released
    while [ -d "$LOCK_FILE" ]; do
        sleep 5  # Check every 5 seconds
    done

    # After the download, proceed with extraction
    echo "Snapshot downloaded by another container. Checking for backup file..."

    if [ -f "$BACKUP_HASH_FILE" ]; then
        echo "Proceeding to extraction for this container."
        tar -xzvf "$BACKUP_FILE" -C /root/.bitcoin
        echo "Extraction complete."
    else
        echo "Expected backup file $BACKUP_FILE does not exist. Unable to proceed with extraction."
        exit 1
    fi
fi

cleanup  # Clean up the lock file if it was acquired
