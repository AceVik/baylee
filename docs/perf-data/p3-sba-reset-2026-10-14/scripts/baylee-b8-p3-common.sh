set -euo pipefail
ROOT=/Users/benchmark-user/.codex/worktrees/b8-perf-cleanup/baylee
OUT=/tmp/baylee-sba-benchmark
TARGET=/Users/benchmark-user/Projects/baylee/target
TOOL_BIN=$(dirname "$(/Users/benchmark-user/.cargo/bin/rustup which rustc)")
cd "$ROOT"
mkdir -p "$OUT" "$OUT/config" "$OUT/cache"

# Preserve the real HOME; discard incidental compiler, allocator, model and
# Criterion runner variables. Repository .cargo/config.toml still applies.
clean() {
    env -i HOME="$HOME" PATH="$TOOL_BIN:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin" \
        CARGO_HOME=/Users/benchmark-user/.cargo RUSTC="$TOOL_BIN/rustc" \
        CARGO_TARGET_DIR="$TARGET" CARGO_INCREMENTAL=0 CARGO_TERM_COLOR=never \
        CARGO_PROFILE_BENCH_DEBUG=line-tables-only \
        XDG_CONFIG_HOME="$OUT/config" XDG_CACHE_HOME="$OUT/cache" \
        "$@"
}

quiet_machine() {
    python3 - <<'PY'
import os, time
end = time.monotonic() + 300
while True:
    load = os.getloadavg()[0]
    if load >= 10:
        raise SystemExit(f"refusing: load={load:.2f} >= 10")
    if load < 4:
        break
    if time.monotonic() >= end:
        raise SystemExit(f"not quiet after 5 min: load={load:.2f}")
    time.sleep(10)
print(f"preflight load={load:.2f}", flush=True)
PY
}

seal_executable() {
    local manifest=$1 name=$2 kind=$3 destination=$4 executable
    executable=$(jq -er --arg name "$name" --arg kind "$kind" '
        select(.reason == "compiler-artifact" and .target.name == $name
          and (.target.kind | index($kind)) and .executable != null)
        | .executable' "$manifest" | sort -u)
    test -n "$executable"
    test -f "$executable"  # also refuses an ambiguous multiline result
    test ! -e "$destination"
    cp -p "$executable" "$destination"
    if test -d "$executable.dSYM"; then
        cp -R "$executable.dSYM" "$destination.dSYM"
    fi
    chmod a-w "$destination"
    /usr/bin/otool -L "$destination" > "$destination.libs.txt"
    shasum -a 256 "$destination" > "$destination.sha256"
}

build_variant() {
    local variant=$1 dir="$OUT/$1"
    mkdir "$dir"  # refuses overwriting an existing sealed variant
    clean "$TOOL_BIN/rustc" -vV > "$dir/rustc.txt"
    clean "$TOOL_BIN/cargo" --version > "$dir/cargo.txt"
    git rev-parse HEAD > "$dir/source-sha"
    git --no-pager diff --binary > "$dir/worktree.patch"
    cp crates/baylee-engine/src/sba.rs "$dir/sba.rs"
    shasum -a 256 Cargo.toml Cargo.lock rust-toolchain.toml .cargo/config.toml \
        crates/baylee-engine/Cargo.toml crates/baylee-engine/benches/basics.rs \
        crates/baylee-engine/benches/basics/rally.rs > "$dir/inputs.sha256"

    clean "$TOOL_BIN/cargo" bench --locked --offline -p baylee-engine \
        --bench basics --profile bench --no-run --message-format=json-render-diagnostics \
        > "$dir/bench-build.jsonl" 2> "$dir/bench-build.log"
    seal_executable "$dir/bench-build.jsonl" basics bench "$dir/basics"

    # Pool dumping is untimed; use the warm dev target for xtask on both sides.
    clean "$TOOL_BIN/cargo" build --locked --offline -p xtask --bin xtask \
        --message-format=json-render-diagnostics \
        > "$dir/xtask-build.jsonl" 2> "$dir/xtask-build.log"
    seal_executable "$dir/xtask-build.jsonl" xtask bin "$dir/xtask"
    cmp "$dir/sba.rs" crates/baylee-engine/src/sba.rs
    shasum -a 256 -c "$dir/inputs.sha256"
    clean "$dir/xtask" pool-dump --out "$dir/pool.txt" > "$dir/pool.log"
    shasum -a 256 "$dir/pool.txt" > "$dir/pool.sha256"
    chmod -R a-w "$dir"
}

measure() {
    local variant=$1 label=$2 samples=$3 warm=$4 seconds=$5 filter=${6-}
    local dir="$OUT/runs/$label"
    mkdir -p "$OUT/runs"
    mkdir "$dir"  # each run keeps its own Criterion tree
    quiet_machine > "$dir/load-before.txt"
    shasum -a 256 -c "$OUT/$variant/basics.sha256"
    clean env CRITERION_HOME="$dir/criterion" /usr/bin/time -l \
        "$OUT/$variant/basics" --bench --noplot --save-baseline "$label" \
        --sample-size "$samples" --warm-up-time "$warm" \
        --measurement-time "$seconds" "$filter" \
        > "$dir/stdout.log" 2> "$dir/stderr-time.log"
    python3 -c 'import os; print(os.getloadavg())' > "$dir/load-after.txt"
    shasum -a 256 -c "$OUT/$variant/basics.sha256"
}
