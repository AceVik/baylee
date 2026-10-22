#!/bin/bash
# The macOS package rows' runner: one throwaway Tart VM per GitHub Actions
# job, on the owner's Mac (docs/releasing.md §"The macOS runner").
#
#   runner.sh prepare   build the VM image the jobs are cloned from (once,
#                       and again when the actions runner is a month old)
#   runner.sh serve     the loop the LaunchAgent keeps running
#   runner.sh install   copy this script to ~/baylee-ci and load the agent
#
# `serve` asks GitHub every POLL_SECS whether a job labelled baylee-vm is
# queued. If one is, it takes the machine's cargo lock (so a CI build and a
# `cargo dev land` never share the CPU: under load the e2e timing tests fail)
# and, while jobs are queued, gives each a fresh VM cloned from the image, two
# at a time (macOS allows two VMs per Mac, and a main push has two macOS
# packages), with a just-in-time registration for exactly that job; the VM is
# deleted when the job is over. Nothing a job leaves behind reaches the next
# one, and Softnet keeps the VMs off the home network.
#
# GitHub is asked with a fine-grained token of its own (this repository only;
# Administration: read and write, Actions: read), which the owner keeps in the
# login keychain under the service name below. The registration it buys goes
# into the VM on stdin and is never written to a file, an argument or a log.
#
# /bin/bash is 3.2 on macOS: no arrays under `set -u`, no here-strings
# (they are temporary files there).
set -u

REPO=${BAYLEE_RUNNER_REPO:-AceVik/baylee}
IMAGE=${BAYLEE_RUNNER_IMAGE:-baylee-runner}
BASE=${BAYLEE_RUNNER_BASE:-ghcr.io/cirruslabs/macos-tahoe-base:latest}
LOCK=${BAYLEE_CARGO_LOCK:-$HOME/.baylee-locks/with-cargo-lock.sh}
KEYCHAIN_SERVICE=baylee-macos-runner
LABEL=baylee-vm
POLL_SECS=60
# A registered runner that has not started a job by then never will: the job
# was cancelled or taken while this one waited for the lock.
IDLE_SECS=900
# No job outlives this: the package rows' timeout is 150 minutes.
JOB_CAP_SECS=10800
MAX_VMS=1
# The dist build keeps two or three cores busy on average (user over real,
# 22.10.2026), so two VMs of eight share the host's ten well. It peaked at
# 9.6 GB (thin LTO of the client); 16 GB holds it and the guest's own macOS.
VM_CPUS=8
VM_MEMORY_MB=16384
VM_DISK_GB=120
# actions/runner, pinned with the digest its release notes print.
RUNNER_VERSION=2.338.0
RUNNER_SHA256=df4cebda25c86a886ed204e49fee63f5c2e7cec5f447b5c98440a826bbdf9df2
HERE=$(cd "$(dirname "$0")" && pwd)
export PATH=/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin

log() { printf '%s %s\n' "$(date '+%F %T')" "$*"; }

token() {
    security find-generic-password -s "$KEYCHAIN_SERVICE" -w 2>/dev/null
}

# How many jobs of unfinished runs are waiting for this runner? (0 when
# GitHub does not answer.)
queued() {
    local run k n=0
    for run in $(gh api "repos/$REPO/actions/runs?status=in_progress&per_page=20" --jq '.workflow_runs[].id') \
        $(gh api "repos/$REPO/actions/runs?status=queued&per_page=20" --jq '.workflow_runs[].id'); do
        k=$(gh api "repos/$REPO/actions/runs/$run/jobs?filter=latest&per_page=100" \
            --jq "[.jobs[] | select(.status == \"queued\" and (.labels | index(\"$LABEL\") != null))] | length")
        n=$((n + ${k:-0}))
    done
    echo "$n"
}

# Runs "$@" for at most $1 seconds. `tart exec` can hang for good when a guest
# wedges (22.10.2026: a second VM's exec never returned and held the cargo
# lock for half an hour), and nothing under the lock may wait unbounded.
bounded() {
    local secs=$1 pid guard status
    shift
    "$@" &
    pid=$!
    (
        sleep "$secs"
        kill "$pid" 2>/dev/null
    ) &
    guard=$!
    wait "$pid" 2>/dev/null
    status=$?
    kill "$guard" 2>/dev/null
    wait "$guard" 2>/dev/null
    return "$status"
}

wait_for_agent() {
    local vm=$1
    for _ in $(seq 1 90); do
        if bounded 10 tart exec "$vm" true >/dev/null 2>&1; then
            return 0
        fi
        sleep 2
    done
    log "$vm: the guest agent never answered"
    return 1
}

# The agent answers before the network does. A VM that cannot reach GitHub
# gets no registration: its job stays queued for the next VM instead of
# failing in its checkout.
wait_for_network() {
    local vm=$1
    for _ in $(seq 1 30); do
        if bounded 10 tart exec "$vm" /usr/bin/curl -sS -m 3 -o /dev/null https://github.com >/dev/null 2>&1; then
            return 0
        fi
        sleep 2
    done
    log "$vm: no way to github.com after 60 s"
    return 1
}

# One VM, one job; $1 tells the VMs of a batch apart. Run it in a subshell:
# the EXIT trap is what deletes the VM.
one_job() {
    local vm reply runner_id tart_pid watch_pid job_pid status
    vm="$LABEL-$(date +%Y%m%d-%H%M%S)-$1"
    tart clone "$IMAGE" "$vm" || return 1
    tart run "$vm" --no-graphics --net-softnet >"$HOME/Library/Logs/$vm.log" 2>&1 &
    tart_pid=$!
    # shellcheck disable=SC2064 # the names are fixed now, on purpose
    trap "tart stop '$vm' >/dev/null 2>&1; wait $tart_pid 2>/dev/null; tart delete '$vm'; rm -f '$HOME/Library/Logs/$vm.log'" EXIT
    wait_for_agent "$vm" || return 1
    wait_for_network "$vm" || return 1
    reply=$(gh api -X POST "repos/$REPO/actions/runners/generate-jitconfig" \
        -f name="$vm" -F runner_group_id=1 -f work_folder=_work \
        -f 'labels[]=self-hosted' -f 'labels[]=macOS' -f 'labels[]=ARM64' -f "labels[]=$LABEL") || return 1
    runner_id=$(printf '%s' "$reply" | jq -r .runner.id)
    log "$vm: registered as runner $runner_id"
    # The job runs in the background so that `wait`, which a signal ends,
    # stands between it and this shell; the runner's own lines are marked
    # with the VM they came from.
    (
        printf '%s\n' "$reply" | jq -r .encoded_jit_config | tart exec -i "$vm" /Users/admin/start-runner.sh 2>&1 |
            while IFS= read -r line; do printf '%s %s\n' "$vm:" "$line"; done
        exit "${PIPESTATUS[2]}"
    ) &
    job_pid=$!
    # A runner that got no job, or a job past any timeout, is stopped; if the
    # exec still does not return, it is killed.
    (
        sleep "$IDLE_SECS"
        if [ "$(gh api "repos/$REPO/actions/runners/$runner_id" --jq .busy 2>/dev/null)" = true ]; then
            sleep $((JOB_CAP_SECS - IDLE_SECS))
            log "$vm: still running after ${JOB_CAP_SECS}s, stopping"
        else
            log "$vm: no job after ${IDLE_SECS}s, stopping"
        fi
        tart stop "$vm" >/dev/null 2>&1
        sleep 30
        pkill -P "$job_pid" 2>/dev/null
        kill "$job_pid" 2>/dev/null
    ) &
    watch_pid=$!
    wait "$job_pid"
    status=$?
    log "$vm: runner exited ($status)"
    kill "$watch_pid" 2>/dev/null
    # A just-in-time runner removes itself after its job; one that never got
    # a job stays registered, offline.
    gh api -X DELETE "repos/$REPO/actions/runners/$runner_id" >/dev/null 2>&1 || true
}

# Runs under the cargo lock: while jobs are queued, up to MAX_VMS at a time,
# each in a VM of its own. A main push that lands meanwhile is served before
# the lock goes back.
batch() {
    local n i pids pid served failed=0
    while true; do
        n=$(queued)
        if [ "${n:-0}" -le 0 ]; then
            break
        fi
        if [ "$n" -gt "$MAX_VMS" ]; then
            n=$MAX_VMS
        fi
        log "$n job(s) queued"
        pids=
        for i in $(seq 1 "$n"); do
            (one_job "$i") &
            pids="$pids $!"
        done
        served=0
        for pid in $pids; do
            if wait "$pid"; then
                served=1
            fi
        done
        # Three rounds in which no VM got as far as a job hand the lock back:
        # a VM that never reaches GitHub must not keep every land waiting.
        if [ "$served" = 1 ]; then
            failed=0
        else
            failed=$((failed + 1))
            if [ "$failed" -ge 3 ]; then
                log "three rounds without a job; giving the lock back"
                break
            fi
        fi
        sleep 10
    done
}

serve() {
    GH_TOKEN=$(token)
    if [ -z "$GH_TOKEN" ]; then
        log "no token in the keychain under $KEYCHAIN_SERVICE"
        exit 1
    fi
    export GH_TOKEN
    log "serving $REPO as $LABEL from $IMAGE"
    while true; do
        if [ "$(queued)" -gt 0 ]; then
            log "a job is queued; taking the cargo lock"
            BAYLEE_SESSION="macos-runner (CI jobs)" "$LOCK" "$0" batch
        fi
        sleep "$POLL_SECS"
    done
}

# What the image runs inside: one job, whose registration arrives on stdin.
# The runner reads ACTIONS_RUNNER_INPUT_JITCONFIG like its --jitconfig flag.
START_RUNNER='#!/bin/bash
set -eu
IFS= read -r ACTIONS_RUNNER_INPUT_JITCONFIG
export ACTIONS_RUNNER_INPUT_JITCONFIG
# A just-in-time runner never ran config.sh, which is what writes .path: the
# job steps get this PATH, rustup first, as on the hosted images.
export PATH=/Users/admin/.cargo/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin
cd /Users/admin/actions-runner
exec ./run.sh'

prepare() {
    if tart list --quiet --source local | grep -qx "$IMAGE"; then
        log "$IMAGE exists; delete it first (tart delete $IMAGE)"
        exit 1
    fi
    tart clone "$BASE" "$IMAGE" || exit 1
    tart set "$IMAGE" --cpu "$VM_CPUS" --memory "$VM_MEMORY_MB" --disk-size "$VM_DISK_GB" || exit 1
    tart run "$IMAGE" --no-graphics >/dev/null 2>&1 &
    local tart_pid=$!
    wait_for_agent "$IMAGE" || exit 1
    tart exec "$IMAGE" /bin/bash -lc "
        set -eux
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
            | sh -s -- -y --profile minimal --default-toolchain stable --target x86_64-apple-darwin
        mkdir -p ~/actions-runner && cd ~/actions-runner
        curl -fsSLo runner.tar.gz https://github.com/actions/runner/releases/download/v$RUNNER_VERSION/actions-runner-osx-arm64-$RUNNER_VERSION.tar.gz
        echo '$RUNNER_SHA256  runner.tar.gz' | shasum -a 256 -c
        tar xzf runner.tar.gz && rm runner.tar.gz
        /usr/bin/arch -x86_64 /usr/bin/true
    " || exit 1
    printf '%s\n' "$START_RUNNER" | tart exec -i "$IMAGE" /bin/bash -c \
        'cat > /Users/admin/start-runner.sh && chmod 755 /Users/admin/start-runner.sh' || exit 1
    tart stop "$IMAGE"
    wait "$tart_pid" 2>/dev/null
    log "$IMAGE is ready"
}

install_agent() {
    local dir=$HOME/baylee-ci plist=$HOME/Library/LaunchAgents/org.baylee.macos-runner.plist
    mkdir -p "$dir"
    install -m 755 "$HERE/runner.sh" "$dir/runner.sh"
    sed "s|@HOME@|$HOME|g" "$HERE/org.baylee.macos-runner.plist" >"$plist"
    launchctl bootout "gui/$(id -u)" "$plist" 2>/dev/null
    launchctl bootstrap "gui/$(id -u)" "$plist"
    log "loaded $plist"
}

case ${1:-} in
prepare) prepare ;;
serve) serve ;;
batch) batch ;;
install) install_agent ;;
*)
    echo "usage: runner.sh prepare|serve|install" >&2
    exit 2
    ;;
esac
