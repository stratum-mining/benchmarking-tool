#!/bin/sh

if [ -z "$1" ] || [ -z "$2" ]; then
    echo "Usage: $0 <TARGET_IP1> <DIVISOR> [TARGET_IP2] [-proxy-type=pool-translator]"
    exit 1
fi

TARGET_IP1="$1"
DIVISOR="$2"
TARGET_IP2=""
PROXY_TYPE=""

# Process optional arguments
for ARG in "$@"; do
    if [ "$ARG" = "-proxy-type=pool-translator" ]; then
        PROXY_TYPE="$ARG"
    elif [ "$ARG" != "$1" ] && [ "$ARG" != "$2" ]; then
        TARGET_IP2="$ARG"
    fi
done

# URL of the Prometheus query
PROMETHEUS_QUERY_URL="http://10.5.0.9:9090/api/v1/query?query=average_pool_subscription_latency_milliseconds"

# Initialize the previous latency variable
PREV_LATENCY=""

while true; do
    # Get the latency value from the Prometheus query
    JSON=$(curl -s "$PROMETHEUS_QUERY_URL")
    LATENCY=$(echo "$JSON" | grep -o '"value":[[0-9.]*,"[0-9.]*' | cut -d',' -f2 | tr -d '"')
    
    # Check if LATENCY is empty
    if [ -z "$LATENCY" ]; then
        echo "No latency value found, skipping this cycle."
    else
        # Compare the current latency with the previous latency
        if [ "$LATENCY" != "$PREV_LATENCY" ]; then
            # Set the latency using the tc command, dividing the value obtained from the query by the divisor
            ADJUSTED_LATENCY=$(echo "$LATENCY $DIVISOR" | awk '{print $1 / $2}')
            echo "Setting latency to ${ADJUSTED_LATENCY}ms"

            if [ "$PROXY_TYPE" = "-proxy-type=pool-translator" ]; then
                # Remove existing qdisc if any
                tc qdisc del dev eth0 root 2>/dev/null
                
                # Apply latency to traffic going to TARGET_IP1 with pool-translator settings
                tc qdisc add dev eth0 root handle 1: prio
                tc qdisc add dev eth0 parent 1:1 handle 10: netem delay "${ADJUSTED_LATENCY}ms"
                iptables -t mangle -F
                iptables -t mangle -A OUTPUT -p tcp -s 10.5.0.17 -d "$TARGET_IP1" --tcp-flags PSH PSH -j MARK --set-mark 1
                tc filter add dev eth0 protocol ip parent 1:0 prio 1 handle 1 fw classid 1:1
            else
                # Remove existing qdisc if any
                tc qdisc del dev eth0 root 2>/dev/null

                # Apply latency to traffic going to TARGET_IP1
                tc qdisc add dev eth0 root handle 1: prio
                tc qdisc add dev eth0 parent 1:3 handle 30: netem delay "${ADJUSTED_LATENCY}ms"
                tc filter add dev eth0 protocol ip parent 1:0 prio 1 u32 match ip dst "$TARGET_IP1" flowid 1:3
                
                # Apply to second target IP if provided
                if [ -n "$TARGET_IP2" ]; then
                    tc filter add dev eth0 protocol ip parent 1:0 prio 1 u32 match ip dst "$TARGET_IP2" flowid 1:3
                fi
            fi

            # Update the previous latency value
            PREV_LATENCY="$LATENCY"
        fi
    fi

    # Wait seconds before querying again
    sleep 5
done
