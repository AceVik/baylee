#!/usr/bin/env bash
# The reinforcement-learning loop: league games -> dataset -> a PPO step
# (train_rl.py) -> export -> arena, iteration after iteration. Every stage writes its own
# directory and is skipped when its output exists, so the loop resumes where
# a stop left it (a stage in progress restarts; training resumes from its
# last.pt).
#
#   BASE=~/baylee-data/models/v3-b ITERS=8 GAMES=10000 tools/trainer/rl_loop.sh
#
# GPU=1 plays every v3 net on the GPU in batches (league and arena with
# `--gpu-batch`), which needs a build with `onnx-cuda` and each net's
# fixed-batch exports; the loop makes both.
#
# The two latest former learners join the league of the ones after them,
# beside the house profiles, the frozen yardstick (policy-v1) and itself.
#
# An iteration with its exported net and its arena is finished and skipped
# whole, even without its dataset: a dataset is 7 GB, only the newest is
# kept (`PRUNE=0` keeps them all), and older records may not convert on a
# newer build.
set -euo pipefail
BASE=${BASE:?the first learner: a model dir with net.pt, net.onnx and seen_ids.npy}
ITERS=${ITERS:-8}
GAMES=${GAMES:-10000}
ARENA=${ARENA:-500}   # duplicate deals: 2 net games and 1 house baseline each
TAG=${TAG:-rl}
TEMP=${TEMP:-1.0}
PRUNE=${PRUNE:-1}
# A second arena per iteration on a deck set the league never plays
# (data/decks/<EVAL>, Astra's archetypes): net − house per matchup is the
# yardstick that generalises. Empty: none.
EVAL=${EVAL:-eval}
DATA=${DATA:-$HOME/baylee-data}
REPO=$(cd "$(dirname "$0")/../.." && pwd)
YARDSTICK=${YARDSTICK:-$DATA/models/policy-v1/policy.onnx}
# GPU=1: every v3 net plays on the GPU through a batch server per net
# (`--gpu-batch`, fixed-batch exports beside each net.onnx); v2 stays on the
# CPU. The CUDA libraries are the trainer's venv's.
GPU=${GPU:-0}
GPU_BATCH=${GPU_BATCH:-64}
cd "$REPO"
if [ "$GPU" = 1 ]; then
    FEATURES=onnx-cuda
    NV=$REPO/tools/trainer/.venv/lib/python3.13/site-packages/nvidia
    export LD_LIBRARY_PATH=$NV/cu13/lib:$NV/cudnn/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
    NETFLAGS=(--gpu-batch --threads "${GPU_THREADS:-96}")
else
    FEATURES=onnx
    NETFLAGS=()
fi
cargo build -q --profile selfplay -p baylee-train --bin convert3
cargo build -q --profile selfplay -p baylee-train --features "$FEATURES" --bin league --bin arena
# A model's fixed-batch exports, made from `$2` when it has none yet.
batched() {
    if [ "$GPU" = 1 ] && ! grep -q '"batched"' "$1/net.onnx.json"; then
        (cd tools/trainer && uv run python export_onnx3.py --model "$1" --data "$2" --batch "$GPU_BATCH") \
            >> "$1.log" 2>&1
    fi
}
# The newest dataset left, for nets from before the loop ran on the GPU.
# (Directories only: each dataset has a .log file beside it.)
last_ds=$(ls -d "$DATA"/datasets/d3-"$TAG"-l*/ 2>/dev/null | sort | tail -1 || true)
last_ds=${last_ds%/}
prev=$BASE
older=""
formers=()
for it in $(seq -f %02g 1 "$ITERS"); do
    run=$DATA/runs/$TAG-l$it
    ds=$DATA/datasets/d3-$TAG-l$it
    model=$DATA/models/$TAG-$it
    arena=$DATA/arena/$TAG-$it
    if [ -f "$model/net.onnx" ] && [ -f "$arena/arena.json" ]; then
        formers+=("net:$prev/net.onnx")
        older=$(printf ',%s' "${formers[@]: -2}")
        prev=$model
        continue
    fi
    echo "[rl_loop] iteration $it: learner $prev ($(date +%H:%M))"
    if [ ! -f "$run/summary.json" ]; then
        rm -rf "$run"
        if [ "$GPU" = 1 ]; then
            batched "$prev" "$last_ds"
            for f in "${formers[@]: -2}"; do batched "$(dirname "${f#net:}")" "$last_ds"; done
        fi
        ./target/selfplay/league "${NETFLAGS[@]}" --learner "$prev/net.onnx" --temperature "$TEMP" \
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
    batched "$model" "$ds"
    last_ds=$ds
    if [ "$PRUNE" = 1 ]; then
        for old in "$DATA"/datasets/d3-"$TAG"-l*; do
            if [ "$old" != "$ds" ] && [ -f "$old/dataset.json" ]; then
                rm -rf "$old"
            fi
        done
    fi
    if [ ! -f "$arena/arena.json" ]; then
        rm -rf "$arena"
        ./target/selfplay/arena "${NETFLAGS[@]}" --model "$model/net.onnx" --against expert --deals "$ARENA" --out "$arena" \
            > "$arena.log" 2>&1
    fi
    if [ -n "$EVAL" ] && [ ! -f "$arena-$EVAL/arena.json" ]; then
        rm -rf "$arena-$EVAL"
        ./target/selfplay/arena "${NETFLAGS[@]}" --model "$model/net.onnx" --against expert --decks "$EVAL" \
            --deals "$ARENA" --out "$arena-$EVAL" > "$arena-$EVAL.log" 2>&1
    fi
    echo "[rl_loop] iteration $it done: $(python3 -c "import json;j=json.load(open('$arena/arena.json'));a=j['results']['expert'];d=j['duplicate'];print('vs expert', round(a['win_rate'],3), a['ci95'], '· net - house', round(d['delta'],3), [round(x,3) for x in d['ci95']])")"
    if [ -n "$EVAL" ] && [ -f "$arena-$EVAL/arena.json" ]; then
        echo "[rl_loop]   $EVAL: $(python3 -c "
import json
d=json.load(open('$arena-$EVAL/arena.json'))['duplicate']
m=sorted(d['by_matchup'].items(), key=lambda kv: kv[1]['delta'])
print('net - house', round(d['delta'],3), [round(x,3) for x in d['ci95']], 'over', len(m), 'matchups; worst', [(k.split(' (')[0], round(v['delta'],2)) for k,v in m[:3]], 'best', [(k.split(' (')[0], round(v['delta'],2)) for k,v in m[-3:]])")"
    fi
    echo "[rl_loop]   league: $(python3 -c "import json;print([(v['opponent'][-40:], round(v['learner_score'],3)) for v in json.load(open('$run/summary.json'))['vs']])")"
    # The two latest former learners stay in the league: every worker holds
    # every net it may meet, and each costs it a few hundred megabytes.
    formers+=("net:$prev/net.onnx")
    older=$(printf ',%s' "${formers[@]: -2}")
    prev=$model
done
