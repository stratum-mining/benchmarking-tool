# Solo-CKPool Configuration

This directory contains configuration files for integrating solo-ckpool into the benchmarking tool.

## Files

- `ckpool.conf` - Main solo-ckpool configuration file
- `README.md` - This documentation file

## Configuration Parameters

### Bitcoin Daemon Connection
- **URL**: `10.5.0.21:48330` - Connects through sv1-node-pool-proxy for metrics collection
- **Auth**: Basic authentication credentials (username/password)
- **Notify**: Enables block notifications from Bitcoin daemon

### Difficulty Settings
- **mindiff**: 1 - Minimum difficulty (solo-ckpool limitation: integers only)
- **startdiff**: 42 - Starting difficulty for new miners
- **maxdiff**: 0 - Maximum difficulty (0 = unlimited)

### Network Settings
- **serverurl**: `0.0.0.0:3333` - Stratum server binding
- **zmqblock**: `tcp://10.5.0.16:28332` - ZMQ block notification endpoint

### Logging
- **logdir**: `/var/log/ckpool` - Log file directory
- **sockdir**: `/var/log/ckpool` - Unix socket directory
- **loginterval**: 60 - Log interval in seconds

## Usage Notes

1. **Solo Mining Mode**: Pool runs with `-B` flag for BTCSOLO mode
2. **Address Validation**: Miner usernames must be valid Bitcoin addresses
3. **Fractional Difficulty**: Not supported - minimum difficulty is 1
4. **Network Integration**: Uses existing Bitcoin node through proxy for monitoring

## Testing

Connect miners using:
```bash
# For testnet
./minerd -a sha256d -o stratum+tcp://<host-ip>:3335 -u <bitcoin-address>.solo-ckpool -q -D -P

# Example with valid testnet address
./minerd -a sha256d -o stratum+tcp://127.0.0.1:3335 -u tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8.solo-ckpool -q -D -P
```

## Metrics Integration

Solo-ckpool integrates with the existing SV1 metrics collection system:
- Share submissions, valid/stale shares
- Block template values and propagation times
- Network bandwidth usage (farm and pool level)
- Container resource usage (CPU, memory)

All metrics are collected through the `solo-ckpool-miner-proxy` component.