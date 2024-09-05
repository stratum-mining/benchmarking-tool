#!/bin/sh

NETWORK=$1
TIMESTAMP_FILE="/root/.bitcoin/last_download_timestamp"
BACKUP_BASE_URL="http://75.119.150.111/backup"
DOWNLOAD_INTERVAL_DAYS=1  # 1 day = 24 hours
CHAINSTATE_DIR="/root/.bitcoin/chainstate"
MAX_RETRIES=30  # Number of attempts (1 per minute for 30 minutes)
RETRY_INTERVAL=60  # 60 seconds (1 minute) between attempts

if [ "$NETWORK" = "" ]; then
  echo "Checking if mainnet snapshot update is needed..."

  # Check if the chainstate directory exists
  if [ -d "$CHAINSTATE_DIR" ]; then
    CHAINSTATE_MOD_TIME=$(stat -c %Y "$CHAINSTATE_DIR")  # Last modification time of chainstate directory
    CURRENT_TIME=$(date +%s)  # Current timestamp
    TIME_DIFF=$(( (CURRENT_TIME - CHAINSTATE_MOD_TIME) / 86400 ))  # Difference in days

    # Check if the chainstate directory was updated more than 24 hours ago
    if [ "$TIME_DIFF" -ge "$DOWNLOAD_INTERVAL_DAYS" ]; then
      echo "It has been more than $DOWNLOAD_INTERVAL_DAYS day(s) since the last node run. Proceeding with download..."
    else
      echo "Node has run recently (within $DOWNLOAD_INTERVAL_DAYS day(s)). Skipping download."
      exit 0
    fi
  else
    echo "Chainstate directory $CHAINSTATE_DIR does not exist. This is a new node. Proceeding with download."
  fi

  # Get current UTC date and time in the format used by the backup files
  CURRENT_UTC=$(date -u +"%Y-%m-%d_%H-UTC")
  BACKUP_FILE_NAME="backup_mainnet_blocks_chainstate_$CURRENT_UTC.tar.gz"
  BACKUP_URL="$BACKUP_BASE_URL/$BACKUP_FILE_NAME"
  BACKUP_HASH_URL="$BACKUP_URL.sha256"

  # Paths for files
  BACKUP_FILE="/tmp/$BACKUP_FILE_NAME"
  BACKUP_HASH_FILE="/tmp/$BACKUP_FILE_NAME.sha256"

  retry_count=0
  success=0

  # Retry downloading the backup every minute for up to MAX_RETRIES (30 minutes)
  while [ $retry_count -lt $MAX_RETRIES ]; do
    echo "Attempt $((retry_count + 1)) of $MAX_RETRIES to download the backup..."

    # Try to download the backup file
    if wget $BACKUP_URL -O $BACKUP_FILE && wget $BACKUP_HASH_URL -O $BACKUP_HASH_FILE; then
      echo "Download succeeded. Verifying the downloaded snapshot..."

      cd /tmp/
      # Check the hash
      if sha256sum -c $BACKUP_HASH_FILE; then
        echo "Hash verification succeeded."
        success=1
        break
      else
        echo "Hash verification failed! Retrying..."
        rm -f $BACKUP_FILE $BACKUP_HASH_FILE
      fi
    else
      echo "Download failed! Retrying in $RETRY_INTERVAL seconds..."
    fi

    retry_count=$((retry_count + 1))
    sleep $RETRY_INTERVAL
  done

  if [ $success -eq 0 ]; then
    echo "Failed to download the backup after $MAX_RETRIES attempts. Aborting."
    exit 1
  fi

  echo "Extracting the snapshot into the bitcoin directory..."

  tar -xzvf $BACKUP_FILE -C /root/.bitcoin

  # Remove the downloaded files to save space
  rm -f $BACKUP_FILE $BACKUP_HASH_FILE

  # Update the timestamp file to the current time
  touch $TIMESTAMP_FILE

  echo "Update completed."
else
  echo "NETWORK is not set to 'mainnet'. Skipping update."
fi
