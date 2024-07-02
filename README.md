<h1 align="center">
  <br>
  <a href="https://stratumprotocol.org"><img src="https://github.com/stratum-mining/stratumprotocol.org/blob/660ecc6ccd2eca82d0895cef939f4670adc6d1f4/src/.vuepress/public/assets/stratum-logo%402x.png" alt="SRI" width="200"></a>
  <br>
Stratum V2 - Benchmarking Tool 📏
  <br>
</h1>


## Overview

Stratum V2 is an essential upgrade to the current pooled mining protocol, Stratum. The existing protocol centralizes transaction selection in mining pools, making them potential attack vectors. Stratum V2 decentralizes this by allowing individual miners to create block templates, enhancing security and performance through an encrypted, binary protocol.

To promote the adoption of Stratum V2, a comprehensive benchmarking tool is needed. This tool tests and compares the performance of Stratum V1 and V2 in various mining scenarios, helping the mining industry understand and benefit from the new protocol.

## 🎯 Goals

1. **Enable miners and pool operators to easily test and benchmark SRI configurations**
2. **Facilitate testing, bug reporting, and feedback collection**
    - Provide a testing suite for each SRI configuration
    - Allow easy customization of role configurations
    - Integrate bug reporting mechanisms into the testing tool
3. **Provide a pre-built benchmarking tool for evaluating SV2 protocol performance**
    - Generate benchmark data for each SRI configuration
    - Automatically generate reports containing benchmark data
    - Compare protocol performance between SV2 and SV1
    - Allow external verification of benchmark data documented in the future SV1-SV2 comparison report

## ✨ Features

- **Comprehensive Testing Suite**: Evaluate different SRI configurations with customizable role settings.
- **Automated Benchmarking**: Generate and collect performance data automatically for both Stratum V1 and Stratum V2.
- **Detailed Reporting**: Create detailed reports comparing protocol performance, with easy-to-understand metrics and visualizations.
- **Integrated Bug Reporting**: Facilitate bug reporting and feedback collection directly within the tool.

📚 To dig more into tool's features. or understand how it is built, please have a look at *docs/* and read the [requirements document](./docs/requirements-document.md) or visualize the [system design](./docs/system-design.png) diagram.


## 🐳 Prerequisites

1. Install Docker on your system: https://docs.docker.com/engine/install/

## ⛏️ Getting started

1. Clone the repository
    ```bash
    git clone https://github.com/stratum-mining/benchmarking-tool.git
    cd benchmarking-tool
    ```

2. Choose what Stratum V2 configuration to benchmark
   - **Configuration A**: it runs **every** role, selecting txs and mining on custom jobs
   - **Configuration C**: it doesn't run [Job Declaration Protocol](https://github.com/stratum-mining/sv2-spec/blob/main/06-Job-Declaration-Protocol.md), so it will mine on Pool's block template
  
    Please have a look at https://stratumprotocol.org to better understand the Stratum V2 configurations and decide which one to benchmark.

3. Run the tool using Docker Compose
- To run Configuration A:
     ```bash
     docker compose -f docker-compose-config-a.yaml up -d
     ```
- To run Configuration C:
     ```bash
     docker compose -f docker-compose-config-c.yaml up -d
     ```

4. Point miners to the following endpoints
   - Stratum V1:
   ```bash
      stratum+tcp://<host-ip-address>:3333
   ```
   - Stratum V2:
   ```bash
      stratum+tcp://<host-ip-address>:34255
   ```
   If you don't have a physical miner, you can do tests with CPUMiner.
  Setup the correct CPUMiner for your OS:
    - You can download the binary directly from [here](https://sourceforge.net/projects/cpuminer/files/);
    - Or compile it from [https://github.com/pooler/cpuminer](https://github.com/pooler/cpuminer)

    On the CPUMiner directory:
    
    `./minerd -a sha256d -o stratum+tcp://<host-ip-address>:34255 -q -D -P`

5. Open your browser and navigate to http://localhost:3000/
6. Click on dashboard, selecting the one called "SRI benchmarking tool"
   
<img src="./docs/images/grafana-dashboard.png" alt="grafana-dashboard">
   
7. Explore data, and click on **Report** button (placed in the upper right corner) to download a PDF containing plots and data for the desired timeframe selected.


## 🛣 Roadmap 

The roadmap of this project can be found here: https://docs.google.com/document/d/1CqcvsxGugFjWy4e4Yf6PjxCs2O4puwlFBO6M0TRL4qE/edit#heading=h.h9x57vygfk4q

## 📖 License

This software is licensed under Apache 2.0 or MIT, at your option.