#!/usr/bin/env bash
set -euo pipefail

# (
  sleep 2
  cansend vcan0 123#0011223344556677
  sleep 0.5
  cansend vcan0 321#AABBCCDDEEFF0011
  sleep 0.5
  cansend vcan0 7FF#1122334455667788
  sleep 0.5
  cansend vcan0 123#00FF223344556677
# ) &

# exec cargo run