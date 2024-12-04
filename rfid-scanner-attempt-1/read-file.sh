#!/bin/sh
set -Eeuo pipefail

curl http://10.3.2.186/read-file?name=$1
