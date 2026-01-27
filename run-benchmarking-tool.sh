#!/bin/bash

# Set default values
DEFAULT_CONFIG="A"
DEFAULT_NETWORK="testnet4"
DEFAULT_HASHRATE="10_000_000_000_000.0"
DEFAULT_POOL_SIGNATURE="Stratum V2 SRI Pool"
DEFAULT_TP_MIN_INTERVAL="60"


# Path to .env file
ENV_FILE=".env"

#Binaries Log Level
DEFAULT_LOG_LEVEL="info"

# Function to clean up Docker containers on error
cleanup() {
    echo ""
    echo "${bg_red}${white}${bold}═══════════════════════════════════════════════════════════════════════════════${reset}"
    echo "${bg_red}${white}${bold}  ❌ Error: Setup Failed${reset}"
    echo "${bg_red}${white}${bold}═══════════════════════════════════════════════════════════════════════════════${reset}"
    echo ""
    echo "  An error occurred during the setup process."
    echo "  Stopping any running Docker containers..."
    echo ""
    docker compose -f "docker-compose-config-${CONFIG_LOWER}.yaml" down 2>/dev/null || true
    echo ""
    echo "  ${bold}Next Steps:${reset}"
    echo "    1. Try running the tool again: ${bold}./run-benchmarking-tool.sh${reset}"
    echo "    2. Check Docker is running: ${bold}docker ps${reset}"
    echo "    3. Review error messages above for details"
    echo ""
    echo "  ${bold}Need Help?${reset}"
    echo "    Discord Support: https://discord.com/channels/950687892169195530/1107964065936060467"
    echo ""
    echo "═══════════════════════════════════════════════════════════════════════════════"
    echo ""
    exit 1
}

# Set up trap to catch errors and call cleanup
trap 'cleanup' ERR

# Color definitions
bold=$(tput bold)
underline=$(tput smul)
reset=$(tput sgr0)

# Text colors
red=$(tput setaf 1)
green=$(tput setaf 2)
yellow=$(tput setaf 3)
blue=$(tput setaf 4)
magenta=$(tput setaf 5)
cyan=$(tput setaf 6)
white=$(tput setaf 7)

# Background colors (optional, for tiles)
bg_blue=$(tput setab 4)
bg_cyan=$(tput setab 6)
bg_green=$(tput setab 2)
bg_red=$(tput setab 1)
echo ""
echo "${cyan}${bold}═══════════════════════════════════════════════════════════════════════════════${reset}"
echo "${cyan}${bold}  Stratum V2 Benchmarking Tool - Configuration Selection${reset}"
echo "${cyan}${bold}═══════════════════════════════════════════════════════════════════════════════${reset}"
echo ""
echo "  ${cyan}${bold}Configuration A (JDC - Job Declaration Client):${reset}"
echo "    ${green}•${reset} Runs all Stratum V2 apps (TP, Pool, JDS, JDC, Translator)"
echo "    ${green}•${reset} Miners select transactions and create custom block templates"
echo "    ${green}•${reset} Provides maximum decentralization and miner control"
echo ""
echo "  ${magenta}${bold}Configuration C (Pool-only):${reset}"
echo "    ${yellow}•${reset} Does NOT run Job Declaration Protocol (no JDC/JDS)"
echo "    ${yellow}•${reset} Miners mine on Pool's block template (similar to Stratum V1)"
echo "    ${yellow}•${reset} Simpler setup, but less decentralized"
echo ""
echo "  ${blue}${underline}Learn more:${reset} ${cyan}https://stratumprotocol.org${reset}"
echo ""
echo "${cyan}───────────────────────────────────────────────────────────────────────────────${reset}"
echo ""

# Prompt user to select configuration (A or C) with default value
read -p "${cyan}${bold}Select configuration to benchmark${reset} ${yellow}[A/C, default: A]:${reset} " CONFIG
CONFIG=${CONFIG:-$DEFAULT_CONFIG}
CONFIG=$(echo "$CONFIG" | tr '[:lower:]' '[:upper:]')

# Validate the CONFIG input
if [[ "$CONFIG" != "A" && "$CONFIG" != "C" ]]; then
    echo ""
    echo "${red}${bold}❌ Error:${reset} Invalid configuration. Please enter 'A' or 'C'."
    exit 1
fi

echo ""
echo "${green}✓${reset} Configuration ${cyan}${bold}${CONFIG}${reset} selected"
echo ""

# Prompt user to select network (mainnet, testnet3, or testnet4) with default value
echo "${cyan}───────────────────────────────────────────────────────────────────────────────${reset}"
read -p "${cyan}${bold}Select Bitcoin network${reset} ${yellow}[mainnet/testnet3/testnet4, default: testnet4]:${reset} " NETWORK
NETWORK=${NETWORK:-$DEFAULT_NETWORK}

# Validate the NETWORK input
if [[ "$NETWORK" != "mainnet" && "$NETWORK" != "testnet3" && "$NETWORK" != "testnet4" ]]; then
    echo ""
    echo "${red}${bold}❌ Error:${reset} Invalid network. Please enter 'mainnet', 'testnet3', or 'testnet4'."
    exit 1
fi

echo ""
echo "${green}✓${reset} Network: ${cyan}${bold}${NETWORK}${reset}"
echo ""

# Prompt user for hashrate to use for SV2 with default value
echo "${cyan}───────────────────────────────────────────────────────────────────────────────${reset}"
echo "  ${cyan}${bold}Hashrate Configuration${reset}"
echo "  Enter the ${underline}real hashrate${reset} of your miner that will connect to Stratum V2"
echo "  ${underline}Examples:${reset}"
echo "    • 10 Th/s → enter: 10000000000000 or 10_000_000_000_000"
echo "    • 100 GH/s → enter: 100000000000 or 100_000_000_000"
echo "    • 1 PH/s → enter: 1000000000000000 or 1_000_000_000_000_000"
echo "  ${underline}Note:${reset} You can enter the number with or without underscores/commas"
echo ""
read -p "${cyan}${bold}Enter your miner's hashrate${reset} ${yellow}[hashes/second, default: 10000000000000]:${reset} " hashrate_input
hashrate_input=${hashrate_input:-"10000000000000"}

# Normalize hashrate input: remove all non-digit characters (underscores, commas, spaces, etc.)
hashrate_clean=$(echo "$hashrate_input" | tr -d '_' | tr -d ',' | tr -d ' ' | sed 's/[^0-9]//g')

# Validate that we have a valid number
if ! [[ "$hashrate_clean" =~ ^[0-9]+$ ]] || [[ -z "$hashrate_clean" ]]; then
    echo ""
    echo "${red}${bold}❌ Error:${reset} Invalid hashrate format."
    echo "   Please enter a valid number (e.g., 10000000000000 or 10_000_000_000_000)"
    exit 1
fi

# Format with underscores every 3 digits from right to left, then add .0
# Use awk to format the number with underscores
hashrate=$(echo "$hashrate_clean" | awk '{
    len = length($0)
    result = ""
    pos = len
    while (pos > 0) {
        start = (pos - 3 > 0) ? pos - 2 : 1
        chunk = substr($0, start, pos - start + 1)
        if (result == "") {
            result = chunk
        } else {
            result = chunk "_" result
        }
        pos = start - 1
    }
    print result ".0"
}')

echo ""
echo "${green}✓${reset} Hashrate: ${cyan}${bold}${hashrate}${reset} hashes/second"
echo ""

# Prompt user to check if they want to configure a custom Bitcoin address
echo "${cyan}───────────────────────────────────────────────────────────────────────────────${reset}"
echo "  ${cyan}${bold}Coinbase Transaction Configuration${reset}"
echo "  The coinbase transaction is the first transaction in each block"
echo "  ${underline}Note:${reset} A Bitcoin address is required to customize the coinbase output"
echo ""
read -p "${cyan}${bold}Configure custom Bitcoin address?${reset} ${yellow}[yes/no, default: no]:${reset} " CONFIGURE_ADDRESS
CONFIGURE_ADDRESS=${CONFIGURE_ADDRESS:-"no"}

# Validate the CONFIGURE_ADDRESS input
if [[ "$CONFIGURE_ADDRESS" != "yes" && "$CONFIGURE_ADDRESS" != "no" ]]; then
    echo ""
    echo "${red}${bold}❌ Error:${reset} Invalid input. Please enter 'yes' or 'no'."
    exit 1
fi

# If the user wants to configure the address, prompt for Bitcoin address
if [[ "$CONFIGURE_ADDRESS" == "yes" ]]; then
    echo ""
    echo "  ${bold}Bitcoin Address Format:${reset}"
    echo "  • Mainnet: bc1q..., 1..., 3..."
    echo "  • Testnet: tb1q..., m..., n..."
    echo "  • The address will be formatted as: addr(your_address)"
    echo ""
    read -p "${bold}Enter Bitcoin address:${reset} " BITCOIN_ADDRESS
    
    # Basic validation for Bitcoin address format
    if [[ -z "$BITCOIN_ADDRESS" ]]; then
        echo ""
        echo "${red}${bold}❌ Error:${reset} Bitcoin address cannot be empty."
        exit 1
    fi
    
    # Check if it looks like a valid Bitcoin address (starts with common prefixes)
    if ! [[ "$BITCOIN_ADDRESS" =~ ^(bc1|tb1|1|2|3|m|n|bc|tb) ]]; then
        echo ""
        echo "${yellow}${bold}⚠️  Warning:${reset} Address format doesn't match common Bitcoin patterns."
        echo "   Continuing anyway, but please verify the address is correct."
    else
        echo ""
        echo "${green}✓${reset} Bitcoin address configured"
    fi
else
    echo ""
    echo "${green}✓${reset} Using default testnet address: ${cyan}tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8${reset}"
fi

# Prompt user to customize the pool signature
echo ""
echo "${cyan}───────────────────────────────────────────────────────────────────────────────${reset}"
echo "  ${cyan}${bold}Pool Signature Configuration${reset}"
echo "  The pool signature is inscribed in the coinbase transaction"
echo "  ${underline}Default:${reset} 'Stratum V2 SRI Pool'"
echo ""
read -p "${bold}Customize pool signature?${reset} [yes/no, default: no]: " CUSTOMIZE_SIGNATURE
CUSTOMIZE_SIGNATURE=${CUSTOMIZE_SIGNATURE:-"no"}

if [[ "$CUSTOMIZE_SIGNATURE" == "yes" ]]; then
    echo ""
    read -p "${bold}Enter custom pool signature${reset} [default: 'Stratum V2 SRI Pool']: " POOL_SIGNATURE
    POOL_SIGNATURE=${POOL_SIGNATURE:-$DEFAULT_POOL_SIGNATURE}
    echo ""
    echo "${green}✓${reset} Pool signature: ${cyan}${bold}${POOL_SIGNATURE}${reset}"
else
    POOL_SIGNATURE=$DEFAULT_POOL_SIGNATURE
    echo ""
    echo "${green}✓${reset} Using default pool signature: ${cyan}${bold}${POOL_SIGNATURE}${reset}"
fi

echo ""
echo "${cyan}───────────────────────────────────────────────────────────────────────────────${reset}"
read -p "${cyan}${bold}Select log level${reset} ${yellow}[info/debug/error/warn, default: info]:${reset} " LOG_LEVEL
LOG_LEVEL=${LOG_LEVEL:-$DEFAULT_LOG_LEVEL}
if ! [[ "$LOG_LEVEL" =~ ^(info|debug|error|warn)$ ]]; then
    echo ""
    echo "${red}${bold}❌ Error:${reset} Invalid log level. Please enter: info, debug, error, or warn."
    exit 1
fi

echo ""
echo "${green}✓${reset} Log level: ${cyan}${bold}${LOG_LEVEL}${reset}"
echo ""

# Prompt for template provider minimum interval
echo "${cyan}───────────────────────────────────────────────────────────────────────────────${reset}"
echo "  ${cyan}${bold}Template Provider Update Interval${reset}"
echo "  Minimum time (in seconds) between block template updates for Template Providers"
echo "  ${underline}Note:${reset} Lower values = more frequent updates, higher CPU usage"
echo ""
read -p "${cyan}${bold}Enter template provider interval${reset} ${yellow}[seconds, default: 60]:${reset} " TP_MIN_INTERVAL
TP_MIN_INTERVAL=${TP_MIN_INTERVAL:-$DEFAULT_TP_MIN_INTERVAL}

# Validate the TP_MIN_INTERVAL input (must be a positive integer)
if ! [[ "$TP_MIN_INTERVAL" =~ ^[0-9]+$ ]]; then
    echo ""
    echo "${red}${bold}❌ Error:${reset} Invalid interval format. Please enter a positive integer."
    exit 1
fi

echo ""
echo "${green}✓${reset} Template provider interval: ${cyan}${bold}${TP_MIN_INTERVAL}${reset} seconds"
echo ""
echo "${bg_green}${white}───────────────────────────────────────────────────────────────────────────────${reset}"
echo ""
echo "  ${green}${bold}Starting Docker containers...${reset}"
echo ""

# Initialize coinbase reward script
if [[ "$CONFIGURE_ADDRESS" == "yes" ]]; then
    COINBASE_REWARD_SCRIPT="addr(${BITCOIN_ADDRESS})"
else
    # Default address
    COINBASE_REWARD_SCRIPT="addr(tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8)"
fi

# Determine Bitcoin socket path based on network
if [[ "$NETWORK" == "mainnet" ]]; then
    BITCOIN_SOCKET_PATH="/root/.bitcoin/node.sock"
else
    BITCOIN_SOCKET_PATH="/root/.bitcoin/${NETWORK}/node.sock"
fi

# Create/update the .env file with only the variables used in config templates
{
    echo "# Variables used in config templates"
    echo ""
    echo "# Pool Settings"
    echo "POOL_COINBASE_REWARD_SCRIPT=${COINBASE_REWARD_SCRIPT}"
    echo "POOL_SIGNATURE=${POOL_SIGNATURE}"
    echo ""
    echo "# JDS Settings"
    echo "JDS_COINBASE_REWARD_SCRIPT=${COINBASE_REWARD_SCRIPT}"
    echo ""
    echo "# JDC Settings"
    echo "JDC_COINBASE_REWARD_SCRIPT=${COINBASE_REWARD_SCRIPT}"
    echo ""
    echo "# Translator Proxy Settings"
    echo "TPROXY_MIN_INDIVIDUAL_MINER_HASHRATE=${hashrate}"
    echo ""
    echo "# Docker Compose Environment Variables"
    if [[ "$NETWORK" == "mainnet" ]]; then
        echo "NETWORK="
    else
        echo "NETWORK=${NETWORK}"
    fi
    echo "LOG_LEVEL=${LOG_LEVEL}"
    echo "TP_MIN_INTERVAL=${TP_MIN_INTERVAL}"
} > "$ENV_FILE"

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
echo "${green}${bold}═══════════════════════════════════════════════════════════════════════════════${reset}"
echo "${green}${bold}  ✓ Benchmarking Tool Started Successfully!${reset}"
echo "${green}${bold}═══════════════════════════════════════════════════════════════════════════════${reset}"
echo ""
echo "  ${cyan}${bold}Miner Connection Information:${reset}"
echo ""
echo "  ${yellow}${underline}Stratum V1 (SV1):${reset}"
echo "    ${bold}URL:${reset} ${green}stratum+tcp://<your-host-ip>:3333${reset}"
echo "    ${underline}Username format:${reset} [bitcoin-address].[nickname]"
echo "    ${underline}Example:${reset} ${cyan}tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8.my-miner${reset}"
echo ""
echo "  ${magenta}${underline}Stratum V2 (SV2):${reset}"
echo "    ${bold}URL:${reset} ${green}stratum+tcp://<your-host-ip>:34255${reset}"
echo ""
echo "  ${cyan}${bold}Example CPU Miner Command:${reset}"
echo "    ${white}./minerd -a sha256d -o stratum+tcp://127.0.0.1:3333 \\${reset}"
echo "    ${white}         -q -D -P -u tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8.my-miner${reset}"
echo ""
echo "${bg_blue}${white}───────────────────────────────────────────────────────────────────────────────${reset}"
echo ""
echo "  ${cyan}${bold}📊 Monitoring Dashboard:${reset}"
echo "    ${underline}Grafana:${reset} ${green}http://localhost:3000/d/64nrElFmk/sri-benchmarking-tool${reset}"
echo ""
echo "  ${cyan}${bold}📄 Generate Benchmark Report:${reset}"
echo "    1. Open the Grafana dashboard"
echo "    2. Click the ${yellow}${bold}\"Report\"${reset} button in the top right corner"
echo "    3. Wait a few minutes for the PDF to generate"
echo ""
echo "${bg_blue}${white}───────────────────────────────────────────────────────────────────────────────${reset}"
echo ""
echo "  ${cyan}${bold}💡 Tips:${reset}"
echo "    ${green}•${reset} Monitor container logs: ${yellow}docker compose -f docker-compose-config-${CONFIG_LOWER}.yaml logs -f${reset}"
echo "    ${green}•${reset} Stop the tool: ${yellow}docker compose -f docker-compose-config-${CONFIG_LOWER}.yaml down${reset}"
echo "    ${green}•${reset} View running containers: ${yellow}docker compose -f docker-compose-config-${CONFIG_LOWER}.yaml ps${reset}"
echo ""
echo "${green}═══════════════════════════════════════════════════════════════════════════════${reset}"
echo ""
