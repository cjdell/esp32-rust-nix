#!/bin/sh
set -Eeuo pipefail

cargo espflash save-image --chip esp32s3 ota.img
