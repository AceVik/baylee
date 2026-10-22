#!/usr/bin/env bash
set -euo pipefail

REPO="/home/ace/Projects/baylee"
DATA="/home/ace/baylee-data/v4"
PY="$REPO/tools/trainer/.venv/bin/python"
SYSROOT="/home/ace/.gemini/antigravity-cli/brain/21eede26-838f-4251-b81c-5456b3bc166c/scratch/sysroot"
export PKG_CONFIG_PATH="$SYSROOT/pkgconfig"
export LIBRARY_PATH="$SYSROOT/usr/lib/x86_64-linux-gnu"
export LD_LIBRARY_PATH="$SYSROOT/usr/lib/x86_64-linux-gnu"

cd "$REPO"
mkdir -p "$DATA/runs" "$DATA/datasets" "$DATA/models" "$DATA/eval"

echo "=== [T4.2] Converting Multi-Format Runs (672 games across 2p, 2v2, 3p, 4p, 6p, 8p) via convert4 ==="
rm -rf "$DATA/datasets/d4-main"
./target/selfplay/convert4 \
    --runs "$DATA/runs/t4-1v1,$DATA/runs/t4-2v2,$DATA/runs/t4-3p,$DATA/runs/t4-4p,$DATA/runs/t4-6p,$DATA/runs/t4-8p" \
    --out "$DATA/datasets/d4-main"

if [ ! -f "$DATA/models/v4-phase-a/net.pt" ]; then
    echo "=== [T3] Phase A: Self-Supervised Card DSL & Set-Context Pretraining ==="
    "$PY" tools/trainer/pretrain_dsl4.py \
        --data "$DATA/datasets/d4-main" \
        --out "$DATA/models/v4-phase-a" \
        --d-model 512 --layers 8 --steps 500
fi

echo "=== [T4.3] Phase B: Unrolled Trajectory Distillation (512/8L, Action-Weighted) ==="
rm -rf "$DATA/models/v4-phase-b"
"$PY" tools/trainer/train4.py \
    --data "$DATA/datasets/d4-main" \
    --init "$DATA/models/v4-phase-a" \
    --out "$DATA/models/v4-phase-b" \
    --d-model 512 --layers 8 --batch-size 8 --unroll 16 --thinking-steps 2 \
    --epochs 2 --max-shards 6 --max-steps 500

"$PY" tools/trainer/export_onnx4.py \
    --model "$DATA/models/v4-phase-b" \
    --data "$DATA/datasets/d4-main"

echo "=== [T5] Phase C: Multi-Format Recurrent PPO League Rollouts & RL Update ==="
rm -rf "$DATA/runs/rl1-1v1" "$DATA/runs/rl1-2v2" "$DATA/runs/rl1-4p" "$DATA/datasets/d4-rl1" "$DATA/models/v4-final"
./target/selfplay/eval4 \
    --model "$DATA/models/v4-phase-b/model.onnx" \
    --seats 2 --shape constructed --deals 32 --temperature 0.35 --thinking-steps 2 --no-baseline \
    --record-out "$DATA/runs/rl1-1v1" --out "$DATA/eval/rl1-rollout-1v1.json"
./target/selfplay/eval4 \
    --model "$DATA/models/v4-phase-b/model.onnx" \
    --seats 4 --teams 1,2,1,2 --shape commander --deals 16 --temperature 0.35 --thinking-steps 2 --no-baseline \
    --record-out "$DATA/runs/rl1-2v2" --out "$DATA/eval/rl1-rollout-2v2.json"
./target/selfplay/eval4 \
    --model "$DATA/models/v4-phase-b/model.onnx" \
    --seats 4 --shape commander --deals 16 --temperature 0.35 --thinking-steps 2 --no-baseline \
    --record-out "$DATA/runs/rl1-4p" --out "$DATA/eval/rl1-rollout-4p.json"

./target/selfplay/convert4 \
    --runs "$DATA/runs/rl1-1v1,$DATA/runs/rl1-2v2,$DATA/runs/rl1-4p" \
    --out "$DATA/datasets/d4-rl1"

"$PY" tools/trainer/train_rl4.py \
    --init "$DATA/models/v4-phase-b" \
    --rollout-data "$DATA/datasets/d4-rl1" \
    --out "$DATA/models/v4-final" \
    --batch-size 8 --unroll 16 --thinking-steps 2 --epochs 1 --max-steps 80 --beta-kl 0.04

"$PY" tools/trainer/export_onnx4.py \
    --model "$DATA/models/v4-final" \
    --data "$DATA/datasets/d4-main"

echo "=== [T6] Inference-Time Thinking Depth Sweep (K = 1, 2, 4, 8) ==="
for K in 1 2 4 8; do
    ./target/selfplay/eval4 \
        --model "$DATA/models/v4-final/model.onnx" \
        --seats 2 --shape constructed --against steady --baseline steady \
        --thinking-steps "$K" --deals 24 \
        --out "$DATA/eval/t6-k${K}-1v1.json"
done

echo "=== [T7] Full Multi-Format & Zero-Shot Held-Out Evaluation Suite ==="
# 1v1 Constructed vs steady & sharp
./target/selfplay/eval4 \
    --model "$DATA/models/v4-final/model.onnx" \
    --seats 2 --shape constructed --against sharp --baseline steady \
    --thinking-steps 2 --deals 30 \
    --out "$DATA/eval/t7-1v1-vs-sharp.json"

# 2v2 Team Unified
./target/selfplay/eval4 \
    --model "$DATA/models/v4-final/model.onnx" \
    --seats 4 --teams 1,2,1,2 --shape commander --against steady --baseline steady \
    --thinking-steps 2 --deals 20 \
    --out "$DATA/eval/t7-2v2-teams.json"

# 4p Commander FFA
./target/selfplay/eval4 \
    --model "$DATA/models/v4-final/model.onnx" \
    --seats 4 --shape commander --against steady --baseline steady \
    --thinking-steps 2 --deals 24 \
    --out "$DATA/eval/t7-4p-commander.json"

# 6p & 8p Multiplayer Commander FFA
./target/selfplay/eval4 \
    --model "$DATA/models/v4-final/model.onnx" \
    --seats 6 --shape commander --against steady --baseline steady \
    --thinking-steps 2 --deals 12 \
    --out "$DATA/eval/t7-6p-ffa.json"

./target/selfplay/eval4 \
    --model "$DATA/models/v4-final/model.onnx" \
    --seats 8 --shape commander --against steady --baseline steady \
    --thinking-steps 2 --deals 8 \
    --out "$DATA/eval/t7-8p-ffa.json"

# Zero-Shot Unseen Cards (Held-Out Pool) in 1v1 and 4p Commander
./target/selfplay/eval4 \
    --model "$DATA/models/v4-final/model.onnx" \
    --seats 2 --shape constructed --held-out-cards --against steady --baseline steady \
    --thinking-steps 2 --deals 24 \
    --out "$DATA/eval/t7-zeroshot-1v1.json"

./target/selfplay/eval4 \
    --model "$DATA/models/v4-final/model.onnx" \
    --seats 4 --shape commander --held-out-cards --against steady --baseline steady \
    --thinking-steps 2 --deals 16 \
    --out "$DATA/eval/t7-zeroshot-4p.json"

echo "=== [DONE] Playernet v4 Full Pipeline Complete ==="
