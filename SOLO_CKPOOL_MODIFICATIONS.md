# Solo-CKPool Integration for Benchmarking Tool

This document describes the implementation of GitHub issue #23: integrating solo-ckpool as an SV1 pool option for optimization measurements and comparisons.

## Overview

Solo-ckpool has been successfully integrated into the benchmarking tool to provide additional SV1 pool comparison capabilities. This integration enables performance comparisons between:

- **public-pool** (Node.js-based) vs **solo-ckpool** (C-based)
- **Pool-based solo mining** vs **True solo mining**
- **Different SV1 implementations** under identical conditions

## Architecture

### Network Layout

```
┌─────────────────┬─────────────────┬─────────────────┐
│   Public Pool   │   Solo-CKPool   │      SV2        │
│   (Node.js)     │      (C)        │   (Various)     │
├─────────────────┼─────────────────┼─────────────────┤
│  10.5.0.8:3332  │ 10.5.0.24:3333  │ 10.5.0.4:34254 │
│       ↕         │       ↕         │       ↕         │
│ Pool-Miner      │ Solo-CKPool     │ SV2 Proxies    │
│ Proxy           │ Proxy           │                 │
│ 10.5.0.19:3333  │ 10.5.0.25:3335  │ Multiple ports  │
│       ↕         │       ↕         │       ↕         │
│    Miners       │    Miners       │    Miners       │
└─────────────────┴─────────────────┴─────────────────┘
                   ↕
            Bitcoin Node (10.5.0.16)
            via proxy (10.5.0.21)
```

### Integration Components

1. **solo-ckpool.dockerfile**: Builds solo-ckpool from GitHub source (https://github.com/xyephy/solo-ckpool)
2. **solo-ckpool service**: Runs ckpool in BTCSOLO mode (-B flag)
3. **solo-ckpool-miner-proxy**: Monitors traffic and collects metrics
4. **Configuration files**: Pool settings and Bitcoin daemon connection
5. **Prometheus integration**: Metrics collection and monitoring

## Configuration

### Pool Configuration (`custom-configs/solo-ckpool/ckpool.conf`)

```json
{
  "btcd": [{
    "url": "10.5.0.21:48330",
    "auth": "username",
    "pass": "password", 
    "notify": true
  }],
  "mindiff": 1,
  "startdiff": 42,
  "maxdiff": 0,
  "serverurl": ["0.0.0.0:3333"],
  "logdir": "/var/log/ckpool",
  "sockdir": "/var/log/ckpool",
  "loginterval": 60,
  "update_interval": 30,
  "zmqblock": "tcp://10.5.0.16:28332"
}
```

### Network Endpoints

- **Solo-CKPool Stratum**: `stratum+tcp://<host>:3335`
- **Public Pool Stratum**: `stratum+tcp://<host>:3333` 
- **SV2 Stratum**: `stratum+tcp://<host>:34255`

### Miner Setup

```bash
# Solo-CKPool (requires valid Bitcoin address as username)
./minerd -a sha256d -o stratum+tcp://127.0.0.1:3335 \
  -u tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8.solo-ckpool -q -D -P

# Public Pool (standard username format)  
./minerd -a sha256d -o stratum+tcp://127.0.0.1:3333 \
  -u tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8.public-pool -q -D -P
```

## Key Features

### Solo Mining Capabilities

- **True Solo Mining**: Each miner gets individual coinbase transactions
- **Address Validation**: Usernames must be valid Bitcoin addresses
- **Direct Block Rewards**: 100% of block rewards go to solving miner
- **No Pool Accounting**: Simplified architecture without share tracking

### Performance Characteristics

- **C Implementation**: Optimized low-level performance
- **Memory Efficient**: Minimal RAM usage compared to Node.js pools
- **High Concurrency**: Handles large numbers of connections efficiently  
- **Hardware Acceleration**: Supports optimized SHA256 (AVX2/SSE4)

### Monitoring Integration

All existing SV1 metrics are collected:

- **Share Metrics**: Submitted, valid, stale shares and acceptance rates
- **Latency Metrics**: Job delivery times and block propagation
- **Bandwidth Metrics**: Farm and pool level network usage
- **Resource Metrics**: CPU, memory usage via cadvisor
- **Block Metrics**: Template values and blocks found

## Enhanced Features

### Fractional Difficulty Support ✅

**NEW**: Solo-ckpool now supports fractional difficulty values (including values < 1.0).

**Enhanced Capabilities**: 
- ✅ Support for fractional difficulty values (e.g., 0.1, 0.5, 2.5)
- ✅ Backward compatibility with integer configurations
- ✅ Suitable for low-hashrate testing scenarios
- ✅ Improved benchmarking accuracy for performance comparisons

**Configuration Examples**:
```json
{
  "mindiff": 0.1,           // Minimum difficulty of 0.1 (fractional)
  "startdiff": 0.5,         // Starting difficulty of 0.5 (fractional)  
  "maxdiff": 100.0,         // Maximum difficulty of 100.0
  "shares_per_minute": 15.0 // Target 15 shares per minute for consistent benchmarking
}
```

**Backward Compatibility**:
- Integer values still work: `"mindiff": 1, "startdiff": 42`
- Mixed configurations supported: `"mindiff": 0.5, "startdiff": 10`
- Default shares_per_minute: 18.0 (maintains existing behavior if not specified)

### Shares Per Minute Control ✅

**NEW**: Solo-ckpool now supports configurable share submission rates for standardized benchmarking.

**Key Benefits**:
- ✅ **Consistent benchmarking**: Set identical share rates for SV1 and SV2 comparisons
- ✅ **Flexible testing scenarios**: Easily adjust load characteristics
- ✅ **Real-time adjustment**: Variable difficulty adapts to maintain target rate
- ✅ **Backward compatibility**: Default 18.0 spm maintains existing behavior

**Configuration Examples for Different Scenarios**:
```json
// Low-latency testing
{"shares_per_minute": 5.0}   // 1 share every 12 seconds

// Balanced benchmarking  
{"shares_per_minute": 15.0}  // 1 share every 4 seconds

// High-throughput testing
{"shares_per_minute": 30.0}  // 1 share every 2 seconds
```

**Algorithm Enhancement**:
- Dynamic difficulty adjustment based on `shares_per_minute / 60.0` target rate
- Proportional hysteresis bounds to prevent oscillation
- Configurable timing thresholds based on target share rate

## Limitations and Considerations

### Solo Mining Requirements

- **Valid Addresses**: Miner usernames must be valid Bitcoin addresses for the target network
- **Network Sync**: Requires fully synced Bitcoin node for optimal performance
- **Block Notifications**: Needs ZMQ or blocknotify for real-time updates

## Usage Instructions

### Starting the Benchmarking Tool

```bash
# Configuration A (with local JDC)
docker compose -f docker-compose-config-a.yaml up -d

# Configuration C (pool-managed)  
docker compose -f docker-compose-config-c.yaml up -d
```

### Connecting Miners

Connect miners to different endpoints for comparison:

```bash
# Terminal 1: Solo-CKPool
./minerd -a sha256d -o stratum+tcp://127.0.0.1:3335 \
  -u <valid-bitcoin-address>.solo-ckpool -q -D -P

# Terminal 2: Public Pool
./minerd -a sha256d -o stratum+tcp://127.0.0.1:3333 \
  -u <valid-bitcoin-address>.public-pool -q -D -P

# Terminal 3: SV2 (via translator)
./minerd -a sha256d -o stratum+tcp://127.0.0.1:34255 -q -D -P
```

### Monitoring Results

Access Grafana dashboard: `http://localhost:3000/d/64nrElFmk/sri-benchmarking-tool`

The dashboard will show comparative metrics between:
- Public Pool vs Solo-CKPool performance
- SV1 vs SV2 protocol efficiency
- Resource usage and network characteristics

## Expected Performance Differences

### Solo-CKPool Advantages

1. **Lower Memory Usage**: C implementation more memory efficient
2. **Higher Connection Capacity**: Better handling of concurrent miners
3. **Faster Processing**: Optimized protocol handling and validation
4. **Authentic Solo Mining**: True individual coinbase generation
5. **✅ NEW: Fractional Difficulty**: Support for difficulty < 1.0 with enhanced precision

### Public Pool Advantages  

1. **Feature Rich**: More configuration options and pool features
2. **Detailed Logging**: More comprehensive debugging information
3. **Flexibility**: Easier to modify and customize
4. **Web Interface**: Built-in monitoring and management tools

## Troubleshooting

### Common Issues

1. **Address Validation Errors**
   - Ensure usernames are valid Bitcoin addresses for target network
   - Use correct address format (testnet vs mainnet)

2. **Connection Failures**
   - Check that solo-ckpool service is running: `docker logs solo-ckpool`
   - Verify Bitcoin node connectivity: `docker logs sv1-node-pool-side`

3. **Metrics Not Appearing**
   - Confirm proxy is running: `docker logs solo-ckpool-miner-proxy`
   - Check Prometheus targets: `http://localhost:9090/targets`

### Log Analysis

```bash
# Solo-CKPool logs
docker logs solo-ckpool

# Proxy metrics logs  
docker logs solo-ckpool-miner-proxy

# Bitcoin node logs
docker logs sv1-node-pool-side
```

## Future Enhancements

### Planned Improvements

1. **Fractional Difficulty Support**: Wrapper layer for difficulty scaling
2. **Enhanced Metrics**: Solo-mining specific performance indicators
3. **Configuration Profiles**: Different solo mining scenarios
4. **Multi-Pool Comparison**: Side-by-side analysis of multiple pools

### Integration Opportunities

- **ASIC Testing**: Real hardware performance comparisons
- **Mainnet Benchmarks**: Production environment analysis  
- **Scalability Tests**: Large-scale miner farm simulations
- **Network Analysis**: Latency impact studies across different pools

## Contributing

To modify or extend the solo-ckpool integration:

1. **Configuration Changes**: Edit `custom-configs/solo-ckpool/ckpool.conf`
2. **Docker Modifications**: Update `solo-ckpool.dockerfile` 
3. **Service Changes**: Modify docker-compose files
4. **Metrics Addition**: Extend Prometheus configuration

## References

- [Solo-CKPool Repository](https://bitbucket.org/ckolivas/ckpool-solo)
- [Benchmarking Tool Documentation](./docs/benchmarking-tool-overview.md)
- [SV1 Protocol Specification](https://braiins.com/stratum-v1/docs)
- [GitHub Issue #23](https://github.com/stratum-mining/benchmarking-tool/issues/23)