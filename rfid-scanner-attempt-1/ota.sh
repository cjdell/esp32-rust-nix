#!/bin/sh
set -Eeuo pipefail

cargo espflash save-image --chip esp32s3 ota.img

# curl -X POST --data-binary @ota.img http://10.3.2.186/update
curl -X POST --data-binary @ota.img http://10.3.2.103/update
