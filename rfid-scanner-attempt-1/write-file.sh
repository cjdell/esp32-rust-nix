#!/bin/sh
set -Eeuo pipefail

curl -X POST --data-binary @spiffs/$1 http://10.3.2.186/write-file?name=$1
