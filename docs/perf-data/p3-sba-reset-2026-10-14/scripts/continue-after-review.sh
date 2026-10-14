set -euo pipefail
source /tmp/baylee-b8-p3-common.sh
CONTROL_FILTER='^(?:engine/priority_pass_x4|layers/refresh_over_20k_stack|rally/refresh/1000|rally/refresh/3000|state/clone|state/clone_from|state/snapshot_hash|state/snapshot_hash_3k_tokens|state/zones_clone)$'
measure A controls-A1 30 1 3 "$CONTROL_FILTER"
measure B controls-B 30 1 3 "$CONTROL_FILTER"
measure A controls-A2 30 1 3 "$CONTROL_FILTER"
python3 /tmp/baylee-sba-benchmark-summary.py "$OUT" --output "$OUT/summary-final.json" > "$OUT/summary-final.txt"
source /tmp/baylee-b8-p3-profile-after-cooldown.sh
