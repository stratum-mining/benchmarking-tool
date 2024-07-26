#!/bin/bash

# Set default values
DEFAULT_CONFIG="A"
DEFAULT_NETWORK="testnet4"
DEFAULT_HASHRATE="10_000_000_000_000.0"

# Default interval based on configuration
DEFAULT_INTERVAL_A="30"
DEFAULT_INTERVAL_C="60"

# Display a note about the configurations
bold=$(tput bold)
underline=$(tput smul)
reset=$(tput sgr0)
echo ""
echo -e "🚨 ${bold}Note:${reset}"
echo -e "${bold}Configuration A:${reset} it runs every role, selecting txs and mining on custom jobs"
echo -e "${bold}Configuration C:${reset} it doesn't run Job Declaration Protocol, so it will mine on Pool's block template"
echo ""
echo "Please have a look at https://stratumprotocol.org to better understand the Stratum V2 configurations and decide which one to benchmark."
echo ""

# Prompt user to select configuration (A or C) with default value
read -p "Do you want to use configuration A or C? (Enter 'A' or 'C', default is 'A'): " CONFIG
CONFIG=${CONFIG:-$DEFAULT_CONFIG}
CONFIG=$(echo "$CONFIG" | tr '[:lower:]' '[:upper:]')

# Validate the CONFIG input
if [[ "$CONFIG" != "A" && "$CONFIG" != "C" ]]; then
    echo "Invalid configuration choice. Please enter 'A' or 'C'."
    exit 1
fi

# Prompt user to select network (mainnet, testnet3, or testnet4) with default value
echo ""
read -p "Do you want to use mainnet, testnet3, or testnet4? (Enter 'mainnet', 'testnet3', or 'testnet4', default is 'testnet4'): " NETWORK
NETWORK=${NETWORK:-$DEFAULT_NETWORK}

# Validate the NETWORK input
if [[ "$NETWORK" != "mainnet" && "$NETWORK" != "testnet3" && "$NETWORK" != "testnet4" ]]; then
    echo "Invalid network choice. Please enter 'mainnet', 'testnet3', or 'testnet4'."
    exit 1
fi

# Prompt user for hashrate to use for SV2 with default value
echo ""
read -p "Enter the hashrate for SV2 (e.g.: for 10 Th/s you need to enter 10_000_000_000_000.0, default is '10_000_000_000_000.0'): " hashrate
hashrate=${hashrate:-$DEFAULT_HASHRATE}

# Validate the hashrate format (with underscores)
if ! [[ "$hashrate" =~ ^[0-9_]+\.0$ ]]; then
    echo "Invalid hashrate format. Please use underscores for grouping digits (e.g., 10_000_000_000_000.0)."
    exit 1
fi

# Inform the user about the block template update interval and get the interval
echo ""
if [[ "$CONFIG" == "A" ]]; then
    echo "The SV1 pool used in the benchmarking tool will generate a new block template every 60 seconds."
    read -p "How often do you want your local Job Declarator Client (JDC) to produce updated templates? (default is '30'): " SV2_INTERVAL
    DEFAULT_INTERVAL=$DEFAULT_INTERVAL_A
else
    echo "The SV1 pool used in the benchmarking tool will generate a new block template every 60 seconds."
    read -p "How often do you want the SV2 pool to send updated block templates? This value will affect the bandwidth used. (default is '60'): " SV2_INTERVAL
    DEFAULT_INTERVAL=$DEFAULT_INTERVAL_C
fi

# Use default if no input is provided
SV2_INTERVAL=${SV2_INTERVAL:-$DEFAULT_INTERVAL}

# Validate the SV2_INTERVAL input (must be a positive integer)
if ! [[ "$SV2_INTERVAL" =~ ^[0-9]+$ ]]; then
    echo "Invalid interval format. Please enter a positive integer."
    exit 1
fi

# Determine the correct TOML file based on configuration
config_file="./custom-configs/sri-roles/config-${CONFIG}/tproxy-config-${CONFIG}-docker-example.toml"

# Update the TOML file with the new hashrate value, keeping underscores
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS uses -i '' for in-place editing
    sed -i '' "s/min_individual_miner_hashrate = [0-9_]*\.0/min_individual_miner_hashrate = $hashrate/" "$config_file"
    sed -i '' "s/channel_nominal_hashrate = [0-9_]*\.0/channel_nominal_hashrate = $hashrate/" "$config_file"
else
    # Linux uses -i for in-place editing
    sed -i "s/min_individual_miner_hashrate = [0-9_]*\.0/min_individual_miner_hashrate = $hashrate/" "$config_file"
    sed -i "s/channel_nominal_hashrate = [0-9_]*\.0/channel_nominal_hashrate = $hashrate/" "$config_file"
fi

# Export environment variables
export NETWORK
export SV2_INTERVAL

# Run docker-compose with the appropriate configuration file
docker compose -f "docker-compose-config-${CONFIG}.yaml" up -d

# Display final messages
echo ""
echo "⛏️ ${underline}Now point your miner(s) to the SV1 setup:${reset} stratum+tcp://<host-ip-address>:3333"
echo "⛏️ ${underline}And point your miner(s) to the SV2 setup:${reset} stratum+tcp://<host-ip-address>:34255"
echo ""
echo "You can access Grafana dashboard at the following link: http://localhost:3000/d/64nrElFmk/sri-benchmarking-tool 📊"
echo ""
echo "Remember to click on the \"Report\" button placed in the right corner to download a detailed PDF containing your benchmarks data 📄"
echo ""
