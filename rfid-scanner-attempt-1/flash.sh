#!/bin/sh
set -Eeuo pipefail

cargo build -r

cargo espflash partition-table partitions.csv

cargo espflash flash --partition-table partitions.csv -s 8mb

# Find these with "espflash partition-table partitions.csv"
# espflash write-bin 0x410000 components/esp-picotts/pico/lang/en-GB_ta.bin
# espflash write-bin 0x4b0000 components/esp-picotts/pico/lang/en-GB_kh0_sg.bin

# cargo espmonitor /dev/ttyACM0
tio /dev/ttyACM0
