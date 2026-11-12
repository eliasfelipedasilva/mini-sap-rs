#!/usr/bin/env bash
set -euo pipefail

echo "=== Starting Docker Container sap-npl-lowram (6 GB RAM Limit) ==="
docker start sap-npl-lowram

echo "=== Configuring /etc/hosts and starting Sybase ASE 16.0 + SAP ASCS01 & D00 ==="
docker exec sap-npl-lowram bash -c '
grep -q "vhcalnplci.dummy.nodomain" /etc/hosts || echo "172.17.0.2 vhcalnplci.dummy.nodomain vhcalnplci vhcalnplcs.dummy.nodomain vhcalnplcs" >> /etc/hosts

if ! pgrep -f "dataserver" >/dev/null; then
  su - sybnpl -c "/sybase/NPL/ASE-16_0/bin/startserver -f /sybase/NPL/ASE-16_0/install/RUN_NPL -f /sybase/NPL/ASE-16_0/install/RUN_NPL_BS" &
  sleep 12
fi

su - npladm -c "/usr/sap/NPL/ASCS01/exe/sapstartsrv pf=/sapmnt/NPL/profile/NPL_ASCS01_vhcalnplci -D -u npladm" 2>/dev/null || true
su - npladm -c "/sapmnt/NPL/exe/uc/linuxx86_64/sapstartsrv pf=/sapmnt/NPL/profile/NPL_D00_vhcalnplci -D -u npladm" 2>/dev/null || true
sleep 2

su - npladm -c "sapcontrol -nr 01 -function Start"
su - npladm -c "sapcontrol -nr 00 -function Start"
sleep 8

su - npladm -c "sapcontrol -nr 01 -function GetProcessList"
su - npladm -c "sapcontrol -nr 00 -function GetProcessList"
'

echo "=== Injecting Rust Governor (/usr/local/bin/mini-sap-rs) into Container ==="
if [ -f "./mini-sap-rs" ]; then
  docker cp ./mini-sap-rs sap-npl-lowram:/usr/local/bin/mini-sap-rs
  docker exec sap-npl-lowram /usr/local/bin/mini-sap-rs --governor
fi

echo "=== Container Memory Footprint ==="
docker stats --no-stream sap-npl-lowram
