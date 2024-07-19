#!/bin/bash

# Verifica se sono stati forniti l'IP di destinazione e il divisore come argomenti
if [ -z "$1" ] || [ -z "$2" ]; then
    echo "Usage: $0 <TARGET_IP> <DIVISOR>"
    exit 1
fi

TARGET_IP="$1"
DIVISOR="$2"

# URL of the Prometheus query
PROMETHEUS_QUERY_URL="http://10.5.0.9:9090/api/v1/query?query=average_pool_subscription_latency_milliseconds"

# Initialize the previous latency variable
PREV_LATENCY=""

while true; do
    # Get the latency value from the Prometheus query
    JSON=$(curl -s $PROMETHEUS_QUERY_URL)
    LATENCY=$(echo "$JSON" | grep -o '"value":[[0-9.]*,"[0-9.]*' | cut -d',' -f2 | tr -d '"')

    # Check if LATENCY is empty
    if [ -z "$LATENCY" ]; then
        echo "No latency value found, skipping this cycle."
    else
        # Compare the current latency with the previous latency
        if [ "$LATENCY" != "$PREV_LATENCY" ]; then
            # Set the latency using the tc command, dividing the value obtained from the query by the divisor
            ADJUSTED_LATENCY=$(echo "$LATENCY" | awk -v div="$DIVISOR" '{print $1 / div}')
            echo "Setting latency to ${ADJUSTED_LATENCY}ms"

            # Remove existing qdisc if any
            tc qdisc del dev eth0 root 2>/dev/null

            # Apply latency to traffic going to TARGET_IP
            tc qdisc add dev eth0 root handle 1: prio
            tc qdisc add dev eth0 parent 1:3 handle 30: netem delay ${ADJUSTED_LATENCY}ms
            tc filter add dev eth0 protocol ip parent 1:0 prio 1 u32 match ip dst $TARGET_IP flowid 1:3

            # Update the previous latency value
            PREV_LATENCY="$LATENCY"
        else
            echo "Latency value has not changed, skipping update."
        fi
    fi

    # Wait seconds before querying again
    sleep 5
done
