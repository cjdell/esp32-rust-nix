#!/bin/sh
set -Eeuo pipefail

# curl http://10.3.2.186/play?name=$1
curl http://10.3.2.103/play?name=$1
