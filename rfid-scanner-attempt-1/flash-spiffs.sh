#!/bin/sh
set -Eeuo pipefail

mkspiffs -c spiffs -s 0x283000 spiffs.bin

espflash write-bin 0x57d000 spiffs.bin
