#!/bin/bash

# Clean up any leftover PID files
rm -f /var/log/ckpool/*.pid /var/log/ckpool/*.sock

# Start latency monitoring in background
/usr/local/bin/monitor_and_apply_latency.sh 10.5.0.25 2 &

# Start ckpool in foreground
exec /usr/local/bin/ckpool -B -c /etc/ckpool/ckpool.conf