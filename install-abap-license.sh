#!/usr/bin/env bash
set -euo pipefail

LIC_FILE="${1:-$HOME/Downloads/NPL.txt}"
if [ ! -f "$LIC_FILE" ]; then
  echo "License file $LIC_FILE not found. Usage: ./install-abap-license.sh /path/to/NPL.txt"
  exit 1
fi

docker cp "$LIC_FILE" sap-npl-lowram:/tmp/NPL.txt
docker exec sap-npl-lowram bash -c '
chown npladm:sapsys /tmp/NPL.txt
su - npladm -c "saplikey pf=/sapmnt/NPL/profile/NPL_D00_vhcalnplci -install /tmp/NPL.txt"
su - npladm -c "saplikey pf=/sapmnt/NPL/profile/NPL_D00_vhcalnplci -show"
'
