#!/bin/bash

# Set default values
DEFAULT_CONFIG="A"
DEFAULT_NETWORK="testnet4"
DEFAULT_HASHRATE="10_000_000_000_000.0"
DEFAULT_SCRIPT_TYPE="P2WPKH"
DEFAULT_POOL_SIGNATURE="Stratum V2 SRI Pool"

# Default interval based on configuration
DEFAULT_INTERVAL_A="30"
DEFAULT_INTERVAL_C="60"

# Path to .env file
ENV_FILE=".env"

#Binaries Log Level
DEFAULT_LOG_LEVEL="info"

# Function to clean up Docker containers on error
cleanup() {
    echo ""
    echo "An error occurred during the setup process."
    echo "Stopping any running Docker containers..."
    docker compose -f "docker-compose-config-${CONFIG_LOWER}.yaml" down
    echo "Docker containers stopped."
    echo "Please try running the tool again with the command: ./run-benchmarking-tool.sh"
    echo "🚨If the issue persists, please contact the support team for assistance on Discord: https://discord.com/channels/950687892169195530/1107964065936060467"
    exit 1
}

# Set up trap to catch errors and call cleanup
trap 'cleanup' ERR

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

# Prompt user to check if they want to configure the custom public key
echo ""
echo -e "🚨 To customize the coinbase transaction output, a Bitcoin address or descriptor is required."
echo -e "   In SV2 v1.5.0, coinbase outputs use Bitcoin descriptors format."
echo ""
read -p "Do you want to configure your custom address for the coinbase transaction? (yes/no, default is 'no'): " CONFIGURE_KEY
CONFIGURE_KEY=${CONFIGURE_KEY:-"no"}

# Validate the CONFIGURE_KEY input
if [[ "$CONFIGURE_KEY" != "yes" && "$CONFIGURE_KEY" != "no" ]]; then
    echo "Invalid input. Please enter 'yes' or 'no'."
    exit 1
fi

# If the user wants to configure the key, prompt for public key and script type
if [[ "$CONFIGURE_KEY" == "yes" ]]; then
    echo ""
    echo -e "You can provide either:"
    echo -e "  1. A Bitcoin address (e.g., tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8)"
    echo -e "  2. A Bitcoin descriptor (e.g., wpkh(xpub...))"
    echo -e "  3. A public key (will be converted to descriptor format)"
    echo ""
    read -p "Enter your Bitcoin address, descriptor, or public key: " PUBLIC_KEY
    
    # Check if it's already a descriptor or address
    if [[ "$PUBLIC_KEY" =~ ^(addr|wpkh|sh|tr|pk)\( ]]; then
        DESCRIPTOR="$PUBLIC_KEY"
    elif [[ "$PUBLIC_KEY" =~ ^(tb1|bc1|[13]) ]]; then
        # It's an address, wrap it in addr() descriptor
        DESCRIPTOR="addr($PUBLIC_KEY)"
    else
        # It's a public key, ask for script type
        echo ""
        read -p "Enter the script type for your public key (P2PK, P2PKH, P2SH, P2WSH, P2WPKH, P2TR, default is 'P2WPKH'): " SCRIPT_TYPE
        SCRIPT_TYPE=${SCRIPT_TYPE:-$DEFAULT_SCRIPT_TYPE}
        
        # Convert to descriptor
        case "$SCRIPT_TYPE" in
            "P2PK") DESCRIPTOR="pk($PUBLIC_KEY)" ;;
            "P2PKH") DESCRIPTOR="pkh($PUBLIC_KEY)" ;;
            "P2WPKH") DESCRIPTOR="wpkh($PUBLIC_KEY)" ;;
            "P2SH") DESCRIPTOR="sh($PUBLIC_KEY)" ;;
            "P2WSH") DESCRIPTOR="wsh($PUBLIC_KEY)" ;;
            "P2TR") DESCRIPTOR="tr($PUBLIC_KEY)" ;;
            *) echo "Invalid script type. Using P2WPKH as default."; DESCRIPTOR="wpkh($PUBLIC_KEY)" ;;
        esac
    fi
fi

# Prompt user to customize the pool signature
echo ""
read -p "Default pool signature inscribed in coinbase tx is 'Stratum V2 SRI Pool'. Do you want to customize it? (yes/no, default is 'no'): " CUSTOMIZE_SIGNATURE
CUSTOMIZE_SIGNATURE=${CUSTOMIZE_SIGNATURE:-"no"}

if [[ "$CUSTOMIZE_SIGNATURE" == "yes" ]]; then
    echo ""
    read -p "Enter the custom pool signature to use (default is 'Stratum V2 SRI Pool'): " POOL_SIGNATURE
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

echo ""
read -p "Choose the log level to display in the tool? (info, debug, error, or warn, default is 'info'): " LOG_LEVEL
LOG_LEVEL=${LOG_LEVEL:-$DEFAULT_LOG_LEVEL}
if ! [[ "$LOG_LEVEL" =~ ^(info|debug|error|warn)$ ]]; then
    echo "Invalid log level. Please enter one of these: info, debug, error, or warn."
    exit 1
fi

# Define all the configuration files to update
CONFIG_FILES=(
    "custom-configs/sri-roles/config-a/pool-config-a-docker-example.toml"
    "custom-configs/sri-roles/config-a/jds-config-a-docker-example.toml"
    "custom-configs/sri-roles/config-a/jdc-config-a-docker-example.toml"
    "custom-configs/sri-roles/config-c/pool-config-c-docker-example.toml"
)

HASHRATE_CONFIG_FILES=(
    "custom-configs/sri-roles/config-a/tproxy-config-a-docker-example.toml"
    "custom-configs/sri-roles/config-c/tproxy-config-c-docker-example.toml"
)

# Update the TOML files with the new hashrate value, keeping underscores
for config_file in "${HASHRATE_CONFIG_FILES[@]}"; do
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS uses -i '' for in-place editing
        sed -i '' "s/min_individual_miner_hashrate[[:space:]]*=[[:space:]]*[0-9_]*\.0/min_individual_miner_hashrate = $hashrate/" "$config_file"
        # Remove deprecated channel_nominal_hashrate field (removed in v1.5.0)
        sed -i '' "/^[[:space:]]*channel_nominal_hashrate[[:space:]]*=/d" "$config_file"
        sed -i '' "/^[[:space:]]*#.*channel_nominal_hashrate/d" "$config_file"
    else
        # Linux uses -i for in-place editing
        sed -i "s/min_individual_miner_hashrate[[:space:]]*=[[:space:]]*[0-9_]*\.0/min_individual_miner_hashrate = $hashrate/" "$config_file"
        # Remove deprecated channel_nominal_hashrate field (removed in v1.5.0)
        sed -i "/^[[:space:]]*channel_nominal_hashrate[[:space:]]*=/d" "$config_file"
        sed -i "/^[[:space:]]*#.*channel_nominal_hashrate/d" "$config_file"
    fi
done

# Update JDC and Pool configs for custom address using new v1.5.0 descriptor format
if [[ "$CONFIGURE_KEY" == "yes" ]]; then
    for config_file in "${CONFIG_FILES[@]}"; do
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "s|coinbase_reward_script = \"[^\"]*\"|coinbase_reward_script = \"$DESCRIPTOR\"|" "$config_file"
        else
            sed -i "s|coinbase_reward_script = \"[^\"]*\"|coinbase_reward_script = \"$DESCRIPTOR\"|" "$config_file"
        fi
    done
fi

# Update pool signature (only for pool configs, not JDC/JDS in v1.5.0)
POOL_CONFIG_FILES=(
    "custom-configs/sri-roles/config-a/pool-config-a-docker-example.toml"
    "custom-configs/sri-roles/config-c/pool-config-c-docker-example.toml"
)

for config_file in "${POOL_CONFIG_FILES[@]}"; do
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS uses -i '' for in-place editing
        sed -i '' "s/pool_signature = \"[^\"]*\"/pool_signature = \"$POOL_SIGNATURE\"/" "$config_file"
    else
        # Linux uses -i for in-place editing
        sed -i "s/pool_signature = \"[^\"]*\"/pool_signature = \"$POOL_SIGNATURE\"/" "$config_file"
    fi
done

# Update the .env file with the selected values
if [[ "$NETWORK" == "mainnet" ]]; then
    echo -e "NETWORK=\nSV2_INTERVAL=$SV2_INTERVAL\nLOG_LEVEL=$LOG_LEVEL" > "$ENV_FILE"
else
    echo -e "NETWORK=$NETWORK\nSV2_INTERVAL=$SV2_INTERVAL\nLOG_LEVEL=$LOG_LEVEL" > "$ENV_FILE"
fi

# Ensure SV1 pool configuration uses the correct network format
SV1_POOL_ENV="custom-configs/sv1-pool/.env"
if [[ -f "$SV1_POOL_ENV" ]]; then
    if [[ "$NETWORK" == "mainnet" ]]; then
        NEW_NETWORK_VALUE="mainnet"
    else
        NEW_NETWORK_VALUE="testnet"
    fi
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/^NETWORK=.*/NETWORK=$NEW_NETWORK_VALUE/" "$SV1_POOL_ENV"
    else
        sed -i "s/^NETWORK=.*/NETWORK=$NEW_NETWORK_VALUE/" "$SV1_POOL_ENV"
    fi
else
    echo "Warning: SV1 pool .env file not found at $SV1_POOL_ENV"
fi

# Convert CONFIG to lowercase for the filename
CONFIG_LOWER=$(echo "$CONFIG" | tr '[:upper:]' '[:lower:]')

# Start docker container with the appropriate compose file
docker compose -f "docker-compose-config-${CONFIG_LOWER}.yaml" up -d

# Display final messages
echo ""
echo "🔗 ${bold}Available Mining Connection Options:${reset}"
echo ""
echo "1️⃣ ${underline}SV1 Public Pool:${reset} stratum+tcp://<host-ip-address>:3333 ⛏️"
echo "   📋 Traditional Stratum v1 protocol for compatibility testing"
echo ""

if [[ "$CONFIG" == "A" ]]; then
    echo "2️⃣ ${underline}SV2 Translator Proxy:${reset} stratum+tcp://<host-ip-address>:34255 ⛏️"
    echo "   📋 SV2 Translator for backward compatibility with SV1 miners"
    echo ""
    echo "3️⃣ ${underline}SV2 Job Declaration Client (JDC):${reset} stratum2+tcp://<host-ip-address>:34265 ⛏️"
    echo "   📋 Native SV2 protocol with Job Declaration for custom transaction selection"
else
    echo "2️⃣ ${underline}SV2 Translator Proxy:${reset} stratum+tcp://<host-ip-address>:34255 ⛏️"
    echo "   📋 SV2 Translator for pool template mining (Config C)"
    echo ""
    echo "3️⃣ ${underline}SV2 Pool Direct:${reset} stratum2+tcp://<host-ip-address>:34254 ⛏️"
    echo "   📋 Native SV2 protocol direct to pool (Config C - no JDC)"
fi
echo ""
echo "🚨 For SV1, you should use the address format [address].[nickname] as the username in your miner setup."
echo "💡 For example, to configure a CPU miner, you can use: ./minerd -a sha256d -o stratum+tcp://127.0.0.1:3333 -q -D -P -u tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8.sv2-gitgab19"
echo ""
echo "📊 You can access the Grafana dashboard at the following link: http://localhost:3000/d/64nrElFmk/sri-benchmarking-tool"
echo ""
echo "📄 Remember to click on the \"Report\" button placed in the top right corner to download a detailed PDF containing your benchmarks data"
echo "↪️ (it will take some minutes to generate a complete PDF, so please be patient :) )"
echo ""
