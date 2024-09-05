#!/bin/bash

URL=$1

# Execute the call and save the response
RESPONSE=$(curl -s --user username:password --data-binary '{"jsonrpc":"1.0","id":"curltest","method":"getblockchaininfo","params":[]}' -H 'content-type:text/plain;' $URL)

# Check if the response contains an error
if echo "$RESPONSE" | jq -e '.error != null' > /dev/null; then
  ERROR_MESSAGE=$(echo "$RESPONSE" | jq -r '.error.message')
  echo "Error: $ERROR_MESSAGE"
  exit 1
fi

# Extract the number of headers and blocks
HEADERS=$(echo "$RESPONSE" | jq -r '.result.headers')
BLOCKS=$(echo "$RESPONSE" | jq -r '.result.blocks')

# Check if the number of blocks is equal to the number of headers
if [ "$HEADERS" -eq "$BLOCKS" ]; then
  echo "Node is fully synchronized"
  exit 0
else
  echo "Node is not fully synchronized"
  exit 1
fi