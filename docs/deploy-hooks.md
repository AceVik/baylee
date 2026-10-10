# Deploy hooks

A server deploy (`scripts/server/baylee-deploy`, §"Server deploys" in
`docs/releasing.md`) can run an operator's own programs at three fixed points,
so that another local service is updated in lockstep with Baylee. The hooks
are optional and generic: without any, a deploy does exactly what it did
before. Nothing in this repository knows what a hook does.

## The contract

```text
/etc/baylee/deploy-hooks.d/<hook> PHASE FULL_TARGET_COMMIT
PHASE = prepare | before-switch | after-switch
```

- **Arguments.** Exactly two: the phase and the full 40-character lowercase
  commit id being deployed. Nothing else, ever: no token, no path, no option.
- **Environment.** Cleared, then `PATH=/usr/sbin:/usr/bin:/sbin:/bin` and
  `LANG=C.UTF-8`, nothing more. A hook that needs a setting reads it from a
  root-owned file of its own.
- **Working directory** `/`; **standard input** `/dev/null`; **umask** `022`.
- **User.** root.
- **Output.** Standard output and standard error go to
  `/var/log/baylee/deploy-hooks.log` (root, `0600`; the directory is made
  `0750` if missing), with a stamped line before and after each hook. They
  never reach the deployer's log or the journal: the deployer says only the
  phase, a hook's place ("hook 2 of 3") and its exit status. Rotate the file
  with logrotate if your hooks are chatty.
- **Order.** Every hook runs once per phase, one at a time, in byte order of
  the file names (the C locale's order: `10-a`, `20-b`, `9-c`, `Z-x`).
  A hook's place is its index in that order.
- **Names.** Only names matching `^[A-Za-z0-9][A-Za-z0-9_-]*$` run. Every
  other entry is ignored: dotfiles, `name~`, `name.bak`, `name.dpkg-old`,
  `name.dpkg-new`, anything with a dot or a space.
- **No hooks.** A missing `/etc/baylee/deploy-hooks.d`, or one with no name
  matching the pattern, means no hooks: the deploy is the one described in
  `scripts/server/baylee-deploy` without this page.

### What a hook must be

The dispatcher checks all of it before it runs anything, and a phase in which
one check fails runs **nothing** and refuses the deploy (exit 77). A file that
matches the name pattern is never skipped silently.

- `/`, `/etc`, `/etc/baylee` and `/etc/baylee/deploy-hooks.d` are
  directories owned by root and not writable by group or others. None of them
  may be a symbolic link (the walk is `openat` with
  `O_DIRECTORY | O_NOFOLLOW`, one directory below the other).
- Each hook is a regular file (not a link, directory, FIFO or device), owned
  by root, not writable by group or others, executable by its owner.
- A script's interpreter (its `#!` line) is an absolute path, and it and
  every directory above it (after resolving links such as `/bin` →
  `/usr/bin`) are root's and not group/other-writable. So is every directory
  of the hooks' `PATH`.
- The hook is opened once with `O_NOFOLLOW`, checked with `fstat` on that
  open file, and that same file is executed (`/proc/self/fd/N`). Replacing
  the file after the check changes nothing about what runs.

Set it up as root:

```sh
install -d -o root -g root -m 0755 /etc/baylee/deploy-hooks.d
install -o root -g root -m 0755 ./10-example /etc/baylee/deploy-hooks.d/10-example
```

## The phases

| phase | when | what is guaranteed |
| --- | --- | --- |
| `prepare` | the target is resolved to its full commit and built; nothing has been staged, stopped or installed | the old system runs untouched and keeps admitting games |
| `before-switch` | admission is held, the agent is stopped, and **no game is running on any agent** (unless the deploy was forced) | the old gateway still runs; the new seat bridge, gateway and browser client are not installed yet |
| `after-switch` | the new gateway answers `/info` with the target commit and `dirty: false` | admission is still held; the agent and the seat agents have not started; nothing is recorded as deployed |

After `after-switch` passes, the hold comes down, the agent starts, every
`baylee-seathost@*` instance is restarted, and the target is recorded as
deployed. Each phase may run more than once for one target (see
§"Retries"), so a hook must be safe to run again.

A hook says:

| exit | meaning | what the deploy does |
| --- | --- | --- |
| `0` | done | the next hook, then the deploy goes on |
| `75` (`EX_TEMPFAIL`) | not ready yet | stops this run; the next timer run (every minute) asks the same phase again for the same target. At `prepare` nothing has changed; later the hold stays up |
| anything else, a signal, or the phase's time running out | failed | stops; the next timer run does **not** retry. `baylee-deploy finish` by hand runs `prepare` again and continues |

The hooks after a hook that did not exit 0 do not run in that phase.

### Time

All hooks of a phase together have **15 minutes** for `prepare`,
**180 seconds** for `before-switch` and **120 seconds** for `after-switch`.
`prepare` is where to fetch and verify; the other two run while no game can
start. Each hook runs in its own process group. Past the budget the whole
group gets `SIGTERM`, five seconds later `SIGKILL`, and the phase has failed.
Whatever a hook leaves running in its group when it exits is killed too, so a
hook that needs something to keep running starts it as a systemd unit, not in
the background. A process that leaves the group (`setsid`) is not tracked.

## The dispatcher's exit codes

`/usr/local/lib/baylee/run-deploy-hooks` (crate `baylee-deploy-hooks`):

| exit | meaning |
| --- | --- |
| `0` | every hook exited 0, or there are none |
| `1` | a hook exited non-zero (not 75), was killed by a signal, or ran out of time |
| `64` | the arguments were not a phase and a full lowercase commit; nothing ran |
| `70` | the dispatcher could not do its job (the log, starting a hook); nothing more ran |
| `75` | a hook said not ready |
| `77` | refused: a check above failed, or it is not running as root; nothing ran |

The deployer treats `75` as "not ready" and every other non-zero status as a
failure, and logs `deploy hooks: <phase> for <commit> passed | is not ready |
failed (status N)`. The binary reads no environment and has no option or test
switch; its tests build a scratch layout through the library instead.

## Holding admission, and the drain

`before-switch` may stop something a running game needs: a bridge of a
hosted seat, or a service an engine elsewhere talks to. `/health`'s
`games.local_running` counts only this machine's agent, so with hooks the
deploy waits for **every** running game (`games.running`), and keeps new ones
from starting on any agent while it waits:

- In `/etc/baylee/gateway.env`, set `BAYLEE_ADMISSION_HOLD` to a path root
  owns and the gateway's user can read, for example
  `/var/lib/baylee-deploy/admission-hold`, and restart the gateway once.
  While that file exists the gateway starts no game from any agent (`503` to a
  new game, a room's start and a rematch; running games go on), and
  `/health` says `"admission": "held"` (`docs/protocol.md` §"Holding admission
  during a deploy"). Not under a `RuntimeDirectory` the gateway's unit
  removes: the hold has to survive the gateway's restart.
- The deploy reads the path from `gateway.env`, checks before `prepare` that
  the running gateway reports `admission` at all, puts the file up (as root,
  `0644`) right after `prepare` and before the agent stops, and checks that
  `/health` then says `held` before it counts games. It takes the file down
  only after `after-switch` passed.
- Without the setting, or with a gateway too old to report `admission`, a
  deploy with hooks refuses before `prepare` and changes nothing.
- `--force` skips the wait for games (it ends them, as without hooks); it
  never skips a hook, and a failed hook stops a forced deploy too.
- Rooms that are only waiting block for at most `BAYLEE_DEPLOY_ROOM_GRACE`
  minutes, as without hooks; while the hold is up none of them can start.

The hold is one gateway's: every agent's games pass through it. Another
gateway on another machine is not held by this file.

## Retries and recovery

Where a deploy is stands in `/opt/baylee/state/transaction` (`target`,
`phase`, `failed`, `tag`), written beside and renamed over. `baylee-deploy
status` shows it.

- `prepare` not ready: the target stays pending, the next `watch` tries the
  same target again without a new release tag. A newer release replaces a
  target that has not got past `prepare`.
- Once anything has changed (`staged`, `draining`, `switching`, `switched`),
  only that target goes on; a newer release is noticed after it is done, and
  `baylee-deploy stage <other>` is refused.
- After a failure the timer only reports where the deploy stopped. Fix the
  cause and run `baylee-deploy finish` (or `stage <same>`): it runs `prepare`
  again, waits for games again (the hold is still up, so there are none), and
  runs `before-switch`, the switch and `after-switch`.
- A run that died past the gateway's restart starts again from `prepare` on
  the next timer run.
- A failed `after-switch` leaves the new gateway up with admission held, the
  agent and seat agents off, and nothing recorded as deployed. Nothing is
  rolled back: no database of Baylee's is touched by a deploy.
- Removing every hook while a deploy is under way: the next deploy that goes
  through without hooks takes the hold down and forgets the transaction.

## Installing

1. Deploy a release that contains this once **without** hooks. That deploy
   installs the dispatcher root-owned `0755` at
   `/usr/local/lib/baylee/run-deploy-hooks` and, where `visudo -c` accepts it,
   `/etc/sudoers.d/baylee-deploy-hooks` from
   `scripts/server/baylee-deploy-hooks.sudoers`; every later deploy installs
   that release's dispatcher the same way after it went through. With hooks
   but no dispatcher, a deploy refuses at `prepare` and changes nothing.
2. Set `BAYLEE_ADMISSION_HOLD` in `gateway.env` and restart the gateway.
3. Add the hooks.

The deploy user calls the dispatcher with `sudo -n`. The sudoers rule allows
only that binary, only as root, only with a phase and a 40-hex lowercase
commit (`^(prepare|before-switch|after-switch) [0-9a-f]{40}$`, which needs
sudo 1.9.10 or newer), with `env_reset`, `!setenv` and a fixed
`secure_path`. Where sudo is older the file does not validate and is left
out; the dispatcher's own argument check (exit 64) is what stands then, and a
deploy user with a broader rule can still call it.

## Writing a hook

```sh
#!/bin/sh
# /etc/baylee/deploy-hooks.d/10-example (root:root 0755)
set -eu
phase=$1 commit=$2
case $phase in
    prepare)
        # Fetch and verify what this release needs; exit 75 to be asked
        # again in a minute, anything else non-zero to stop the deploy.
        ;;
    before-switch)
        # No game is running and none can start: stop what must stop.
        ;;
    after-switch)
        # The new gateway is up: start what has to run with it.
        ;;
esac
```

What the dispatcher cannot check is the operator's to keep:

- **Run nothing that a non-root user can write.** No script, binary,
  library, configuration or working directory under a home directory, `/tmp`,
  a deploy user's checkout (`/opt/baylee/src` belongs to the deploy user) or
  anything else writable by someone other than root. `PATH` is fixed for this
  reason.
- **Verify every artifact before installing it.** A private artifact is
  checked against a pinned checksum or a signature whose key or digest lives
  in a root-owned file, before it is used, in `prepare`. Install the result
  root-owned and not group/other-writable. Never `curl | sh`.
- **No secrets in arguments or the environment.** Read them from root-only
  files.
- **Be idempotent.** Every phase may run again for the same commit.
- **Use `75` for "not yet"** (an artifact for this commit is not published
  yet), not for errors.
- **Keep `before-switch` and `after-switch` short**; do the slow work in
  `prepare`.

## Signed release tags

`watch` deploys the newest `v*` tag, and `baylee-deploy stage <tag>` a named
one. Either can be required to carry a signature by a key the operator
allows:

- `/etc/baylee/release-signers`: an SSH allowed-signers file
  (`git config gpg.ssh.allowedSignersFile`), and/or
- `/etc/baylee/release-keyring.gpg`: an OpenPGP keyring (`gpg --export`).

Each must be root's, not a link and not group/other-writable, and so must
`/etc/baylee`. When one exists, a tag is deployed only if `git verify-tag`
accepts it against exactly those keys: git runs with an empty GnuPG home (or
one holding only that keyring) and with none of the deploy user's own git
settings. A refused tag is logged, not built, and not tried again every
minute (`last-release` moves on); once the keys are right, `baylee-deploy
stage <tag>` deploys it. With neither file, every tag is taken as before and
the log says it was not verified. A commit id given to `stage` by hand is the
operator's own choice and is not checked.
