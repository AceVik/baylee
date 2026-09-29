#!/usr/bin/env bash
# Stops every training job, copies what they made to C:\baylee-data and
# pushes the branch: the last step on the Windows + WSL PC before the switch
# to Ubuntu (HANDOVER.md). Run it only when the owner says so; it finishes
# in well under 15 minutes.
#
#   tools/trainer/switch_stop.sh
set -uo pipefail
DATA=$HOME/baylee-data
DEST=/mnt/c/baylee-data
REPO=$(cd "$(dirname "$0")/../.." && pwd)
t0=$(date +%s)

echo "[switch] stopping jobs"
pkill -f 'tools/trainer/rl_loop.sh' || true
pkill -f 'target/selfplay/(league|arena|convert3|selfplay|fuzz)' || true
pkill -f 'python (train3|train_rl|export_onnx3)\.py' || true
sleep 5
# Training writes last.pt every 30 minutes; a finished stage keeps its
# output, so rl_loop.sh resumes where this stopped.

echo "[switch] copying to $DEST"
mkdir -p "$DEST"
for d in runs models arena fuzz verify reports; do
    [ -d "$DATA/$d" ] && rsync -a --exclude 'merged' "$DATA/$d" "$DEST/"
done
cp -a "$DATA"/*.log "$DEST/" 2>/dev/null || true
# Datasets are made again from their runs (convert3 is deterministic); only
# the newest league dataset goes along, to spare the first iteration there.
latest=$(ls -d "$DATA"/datasets/d3-rl-l* 2>/dev/null | sort | tail -1)
if [ -n "$latest" ]; then
    mkdir -p "$DEST/datasets"
    rsync -a --exclude 'merged' "$latest" "$DEST/datasets/"
fi
du -sh "$DEST"

echo "[switch] pushing"
cd "$REPO"
git status --short | head -20
git push -q origin c42/trained-ai && git log --oneline -1
echo "[switch] done in $(( $(date +%s) - t0 )) s"
