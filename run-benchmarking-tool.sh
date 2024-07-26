#!/bin/bash

# Set default values
DEFAULT_CONFIG="A"
DEFAULT_NETWORK="testnet4"
DEFAULT_HASHRATE="10_000_000_000_000.0"
DEFAULT_SCRIPT_TYPE="P2WPKH"
DEFAULT_POOL_SIGNATURE="Stratum v2 SRI Pool"

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
read -p "Which Stratum V2 configuration do you want to benchmark? (Enter 'A' or 'C', default is 'A'): " CONFIG
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

# Prompt user to check if the custom public key is already configured
echo ""
echo -e "🚨 To customize the coinbase transaction output, a custom public key (or redeem script) is required."
echo ""
read -p "Have you already configured your custom public key for the coinbase transaction? (yes/no, default is 'no'): " KEY_CONFIGURED
KEY_CONFIGURED=${KEY_CONFIGURED:-"no"}

# Validate the KEY_CONFIGURED input
if [[ "$KEY_CONFIGURED" != "yes" && "$KEY_CONFIGURED" != "no" ]]; then
    echo "Invalid input. Please enter 'yes' or 'no'."
    exit 1
fi

# If the user has not configured the key, prompt for public key and script type
if [[ "$KEY_CONFIGURED" == "no" ]]; then
    echo ""
    echo -e "If you still don't have a public key, setup a new wallet and extract the extended public key it provides. At this point, you can derive the child public key using this script: https://github.com/stratum-mining/stratum/tree/dev/utils/bip32-key-derivation"
    echo ""
    read -p "Now enter the public key (or redeem script) to use for generating the address in the coinbase transaction: " PUBLIC_KEY
    echo ""
    read -p "Enter the script type (P2PK, P2PKH, P2SH, P2WSH, P2WPKH, P2TR, default is 'P2WPKH'): " SCRIPT_TYPE
    SCRIPT_TYPE=${SCRIPT_TYPE:-$DEFAULT_SCRIPT_TYPE}

    # Validate the script type
    VALID_SCRIPT_TYPES=("P2PK" "P2PKH" "P2SH" "P2WSH" "P2WPKH" "P2TR")
    if [[ ! " ${VALID_SCRIPT_TYPES[@]} " =~ " ${SCRIPT_TYPE} " ]]; then
        echo "Invalid script type. Please enter one of the following: P2PK, P2PKH, P2SH, P2WSH, P2WPKH, P2TR."
        exit 1
    fi
fi

# Prompt user to customize the pool signature
echo ""
read -p "Default pool signature inscribed in coinbase tx is 'Stratum v2 SRI Pool'. Do you want to customize it? (yes/no, default is 'no'): " CUSTOMIZE_SIGNATURE
CUSTOMIZE_SIGNATURE=${CUSTOMIZE_SIGNATURE:-"no"}

if [[ "$CUSTOMIZE_SIGNATURE" == "yes" ]]; then
    echo ""
    read -p "Enter the custom pool signature to use (default is 'Stratum v2 SRI Pool'): " POOL_SIGNATURE
    POOL_SIGNATURE=${POOL_SIGNATURE:-$DEFAULT_POOL_SIGNATURE}
else
    POOL_SIGNATURE=$DEFAULT_POOL_SIGNATURE
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

CONFIG=$(echo "$CONFIG" | tr '[:upper:]' '[:lower:]')

# Determine the correct TOML file based on configuration
config_file="./custom-configs/sri-roles/config-${CONFIG}/tproxy-config-${CONFIG}-docker-example.toml"
jdc_config_file="./custom-configs/sri-roles/config-${CONFIG}/jdc-config-${CONFIG}-docker-example.toml"
pool_config_file="./custom-configs/sri-roles/config-${CONFIG}/pool-config-${CONFIG}-docker-example.toml"

# Update the TOML files with the new hashrate value, keeping underscores
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS uses -i '' for in-place editing
    sed -i '' "s/min_individual_miner_hashrate = [0-9_]*\.0/min_individual_miner_hashrate = $hashrate/" "$config_file"
    sed -i '' "s/channel_nominal_hashrate = [0-9_]*\.0/channel_nominal_hashrate = $hashrate/" "$config_file"
    sed -i '' "s/min_individual_miner_hashrate = [0-9_]*\.0/min_individual_miner_hashrate = $hashrate/" "$pool_config_file"
    sed -i '' "s/channel_nominal_hashrate = [0-9_]*\.0/channel_nominal_hashrate = $hashrate/" "$pool_config_file"
    
    if [[ "$KEY_CONFIGURED" == "no" ]]; then
        # Update the jdc and pool config files with the new public key and script type
        sed -i '' "s/\(output_script_type = \"$SCRIPT_TYPE\", output_script_value = \)\"[^\"]*\"/\1\"$PUBLIC_KEY\"/" "$jdc_config_file"
        sed -i '' "s/\(output_script_type = \"$SCRIPT_TYPE\", output_script_value = \)\"[^\"]*\"/\1\"$PUBLIC_KEY\"/" "$pool_config_file"
    fi
    
    # Update pool signature
    sed -i '' "s/pool_signature = \"[^\"]*\"/pool_signature = \"$POOL_SIGNATURE\"/" "$jdc_config_file"
    sed -i '' "s/pool_signature = \"[^\"]*\"/pool_signature = \"$POOL_SIGNATURE\"/" "$pool_config_file"
    
    if [[ "$CONFIG" == "A" ]]; then
        # Configuration A specific file
        sed -i '' "s/pool_signature = \"[^\"]*\"/pool_signature = \"$POOL_SIGNATURE\"/" "$pool_config_file"
    fi
else
    # Linux uses -i for in-place editing
    sed -i "s/min_individual_miner_hashrate = [0-9_]*\.0/min_individual_miner_hashrate = $hashrate/" "$config_file"
    sed -i "s/channel_nominal_hashrate = [0-9_]*\.0/channel_nominal_hashrate = $hashrate/" "$config_file"
    sed -i "s/min_individual_miner_hashrate = [0-9_]*\.0/min_individual_miner_hashrate = $hashrate/" "$pool_config_file"
    sed -i "s/channel_nominal_hashrate = [0-9_]*\.0/channel_nominal_hashrate = $hashrate/" "$pool_config_file"
    
    if [[ "$KEY_CONFIGURED" == "no" ]]; then
        # Update the jdc and pool config files with the new public key and script type
        sed -i "s/\(output_script_type = \"$SCRIPT_TYPE\", output_script_value = \)\"[^\"]*\"/\1\"$PUBLIC_KEY\"/" "$jdc_config_file"
        sed -i "s/\(output_script_type = \"$SCRIPT_TYPE\", output_script_value = \)\"[^\"]*\"/\1\"$PUBLIC_KEY\"/" "$pool_config_file"
    fi
    
    # Update pool signature
    sed -i "s/pool_signature = \"[^\"]*\"/pool_signature = \"$POOL_SIGNATURE\"/" "$jdc_config_file"
    sed -i "s/pool_signature = \"[^\"]*\"/pool_signature = \"$POOL_SIGNATURE\"/" "$pool_config_file"
    
    if [[ "$CONFIG" == "A" ]]; then
        # Configuration A specific file
        sed -i "s/pool_signature = \"[^\"]*\"/pool_signature = \"$POOL_SIGNATURE\"/" "$pool_config_file"
    fi
fi

# Export environment variables
export NETWORK
export SV2_INTERVAL

# Run docker-compose with the appropriate configuration file
docker compose -f "docker-compose-config-${CONFIG}.yaml" up -d

# Display final messages
echo ""
echo "${underline}Now point your miner(s) to the SV1 setup:${reset} stratum+tcp://<host-ip-address>:3333 ⛏️"
echo "${underline}And point your miner(s) to the SV2 setup:${reset} stratum+tcp://<host-ip-address>:34255 ⛏️"
echo ""
echo "🚨 For SV1, you should use the address format [address].[nickname] as the username in your miner setup."
echo "💡 For example, to configure a CPU miner, you can use: ./minerd -a sha256d -o stratum+tcp://127.0.0.1:3333 -q -D -P -u tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8.sv2-gitgab19"
echo ""
echo "📊 You can access the Grafana dashboard at the following link: http://localhost:3000/d/64nrElFmk/sri-benchmarking-tool"
echo ""
echo "📄 Remember to click on the \"Report\" button placed in the top right corner to download a detailed PDF containing your benchmarks data"
echo "↪️ (it will take some minutes to generate a complete PDF, so please be patient :) )"
echo ""
