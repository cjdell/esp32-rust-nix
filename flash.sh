#!/bin/sh

cargo espflash partition-table partitions.csv

cargo espflash flash --partition-table partitions.csv -s 8mb

# espflash write-bin 0x291000 components/esp-picotts/pico/lang/en-GB_ta.bin
# espflash write-bin 0x331000 components/esp-picotts/pico/lang/en-GB_kh0_sg.bin

# cargo espmonitor /dev/ttyACM0
tio /dev/ttyACM0
