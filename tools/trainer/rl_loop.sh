#!/usr/bin/env bash
# The reinforcement-learning loop: league games -> dataset -> an AWR step ->
# export -> arena, iteration after iteration. Every stage writes its own
# directory and is skipped when its output exists, so the loop resumes where
# a stop left it (a stage in progress restarts; training resumes from its
# last.pt).
#
#   BASE=~/baylee-data/models/v3-b ITERS=8 GAMES=10000 tools/trainer/rl_loop.sh
#
# The two latest former learners join the league of the ones after them,
# beside the house profiles, the frozen yardstick (policy-v1) and itself.
set -euo pipefail
BASE=${BASE:?the first learner: a model dir with net.pt, net.onnx and seen_ids.npy}
ITERS=${ITERS:-8}
GAMES=${GAMES:-10000}
ARENA=${ARENA:-500}
TAG=${TAG:-rl}
TEMP=${TEMP:-1.0}
DATA=${DATA:-$HOME/baylee-data}
REPO=$(cd "$(dirname "$0")/../.." && pwd)
YARDSTICK=${YARDSTICK:-$DATA/models/policy-v1/policy.onnx}
cd "$REPO"
cargo build -q --profile selfplay -p baylee-train --bin convert3
cargo build -q --profile selfplay -p baylee-train --features onnx --bin league --bin arena
prev=$BASE
older=""
formers=()
for it in $(seq -f %02g 1 "$ITERS"); do
    run=$DATA/runs/$TAG-l$it
    ds=$DATA/datasets/d3-$TAG-l$it
    model=$DATA/models/$TAG-$it
    arena=$DATA/arena/$TAG-$it
    echo "[rl_loop] iteration $it: learner $prev ($(date +%H:%M))"
    if [ ! -f "$run/summary.json" ]; then
        rm -rf "$run"
        ./target/selfplay/league --learner "$prev/net.onnx" --temperature "$TEMP" \
            --league "house:expert,house:sharp,house:steady,net:$YARDSTICK,self$older" \
            --games "$GAMES" --out "$run" > "$run.log" 2>&1
    fi
    if [ ! -f "$ds/dataset.json" ]; then
        rm -rf "$ds"
        ./target/selfplay/convert3 --runs "$run" --out "$ds" --every 2 > "$ds.log" 2>&1
    fi
    if [ ! -f "$model/net.pt" ]; then
        (cd tools/trainer && uv run python train_rl.py --data "$ds" --init "$prev/net.pt" --out "$model" --resume) \
            > "$model.log" 2>&1
    fi
    if [ ! -f "$model/net.onnx" ]; then
        (cd tools/trainer && uv run python export_onnx3.py --model "$model" --data "$ds") >> "$model.log" 2>&1
    fi
    if [ ! -f "$arena/arena.json" ]; then
        rm -rf "$arena"
        ./target/selfplay/arena --model "$model/net.onnx" --against expert --games "$ARENA" --out "$arena" \
            > "$arena.log" 2>&1
    fi
    echo "[rl_loop] iteration $it done: $(python3 -c "import json;a=json.load(open('$arena/arena.json'))['results']['expert'];print('vs expert', round(a['win_rate'],3), a['ci95'])")"
    echo "[rl_loop]   league: $(python3 -c "import json;print([(v['opponent'][-40:], round(v['learner_score'],3)) for v in json.load(open('$run/summary.json'))['vs']])")"
    # The two latest former learners stay in the league: every worker holds
    # every net it may meet, and each costs it a few hundred megabytes.
    formers+=("net:$prev/net.onnx")
    older=$(printf ',%s' "${formers[@]: -2}")
    prev=$model
done
