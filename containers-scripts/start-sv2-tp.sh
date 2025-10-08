#!/bin/bash

# Start bitcoin-node in the background with IPC binding
/bitcoin/bin/bitcoin-node -chain=testnet4 -ipcbind=unix -server=1 -rpcuser=username -rpcpassword=password -rpcbind=0.0.0.0:18332 -rpcallowip=0.0.0.0/0 &

# Wait a moment for bitcoin to start
sleep 5

# Start sv2-tp in the foreground, connecting to the bitcoin node via IPC
exec /bitcoin/bin/sv2-tp -sv2port="$SV2_PORT" -sv2interval="$SV2_INTERVAL" -sv2feedelta=0 -debug=sv2 -loglevel=sv2:debug -sv2bind=0.0.0.0 -ipcconnect=unix:/root/.bitcoin/testnet4/node.sock