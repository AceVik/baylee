#!/bin/bash
set -euo pipefail
printf '%s\n' 'Cooling down 30 seconds before separate profiling'
sleep 30
source /tmp/baylee-b8-p3-profile.sh
