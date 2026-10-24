#!/usr/bin/env python3
"""Read-only Linux experiment watcher; invoke check periodically with a user timer.

Only root/watchdog is written. Notifications are at least once: a crash after
Codex accepts a wake but before state publication can repeat its stable event ID.
A recorded successful delivery is never retried. No training imports are needed.
"""
import argparse
import contextlib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import uuid


def atomic_json(path, value):
    temporary = path.with_suffix('.tmp')
    with temporary.open('w') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary, path)
    descriptor = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def process_identity(pid, proc=Path('/proc')):
    """A zombie is exited; start ticks remain valid even with spaces in comm."""
    try:
        fields = (proc / str(pid) / 'stat').read_text().rsplit(')', 1)[1].split()
        if fields[0] in ('Z', 'X'):
            return None
        return {'pid': pid, 'start_ticks': int(fields[19]),
                'boot_id': (proc / 'sys/kernel/random/boot_id').read_text().strip()}
    except (OSError, ValueError, IndexError):
        return None


@contextlib.contextmanager
def locked(directory):
    directory.mkdir(parents=True, exist_ok=True)
    with (directory / 'lock').open('a') as stream:
        try:
            fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            yield False
        else:
            try:
                yield True
            finally:
                fcntl.flock(stream, fcntl.LOCK_UN)


def arm(root, thread, success_phases, artifacts, pid=None, poll_seconds=30,
        error_polls=3, death_polls=2, identity=process_identity):
    root = Path(root).resolve()
    uuid.UUID(thread)  # Preserve the exact supplied thread string in config/argv.
    if not success_phases or poll_seconds < 1 or error_polls < 2 or death_polls < 2:
        raise ValueError('phases required; poll >= 1s and confirmation counts >= 2')
    pid = pid if pid is not None else int((root / 'controller.pid').read_text())
    controller = identity(pid)
    if controller is None:
        raise ValueError(f'controller {pid} is not live; refusing to arm')
    config = {'root': str(root), 'thread': thread, 'controller': controller,
              'success_phases': success_phases, 'artifacts': artifacts,
              'poll_seconds': poll_seconds, 'error_polls': error_polls,
              'death_polls': death_polls}
    directory = root / 'watchdog'
    with locked(directory) as acquired:
        if not acquired:
            raise RuntimeError('watcher is busy')
        existing = directory / 'config.json'
        if existing.exists():
            if json.loads(existing.read_text()) != config:
                raise ValueError('watcher already armed with different configuration')
            return config
        atomic_json(directory / 'state.json', {})
        atomic_json(existing, config)
    return config


def queue_wake(thread, message):
    result = subprocess.run(['/usr/local/bin/codex', 'queue', '--thread', thread,
                    '--message', message], check=True, timeout=30,
                   stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    return result.stdout


def check(root, now=time.time, identity=process_identity, queue=queue_wake):
    directory = Path(root).resolve() / 'watchdog'
    with locked(directory) as acquired:
        if not acquired:
            return 'busy'
        config = json.loads((directory / 'config.json').read_text())
        state_path = directory / 'state.json'
        state = json.loads(state_path.read_text())
        if state.get('delivered'):
            atomic_json(directory / 'delivered.json', state['delivered'])
            return 'delivered'
        timestamp = now()
        if state.get('pending'):
            if timestamp < state.get('retry_at', 0):
                return 'backoff'
        else:
            if timestamp < state.get('next_check', 0):
                return 'waiting'
            state['next_check'] = timestamp + config['poll_seconds']
            root = Path(config['root'])
            try:
                status = json.loads((root / 'status.json').read_text())
                if not isinstance(status, dict) or not isinstance(status.get('phase'), str):
                    raise ValueError('status must be an object with a string phase')
                state['status_errors'] = 0
                status_error = None
            except (OSError, ValueError) as error:
                status = None
                status_error = f'{type(error).__name__}: {error}'
                state['status_errors'] = state.get('status_errors', 0) + 1
            live = identity(config['controller']['pid']) == config['controller']
            state['dead_polls'] = 0 if live else state.get('dead_polls', 0) + 1
            phase = status.get('phase') if status else None
            finding = None
            if phase == 'failed':
                finding = 'experiment failed'
            elif phase in config['success_phases']:
                missing = []
                for name in config['artifacts']:
                    artifact = root / name
                    try:
                        if not artifact.is_file():
                            raise ValueError('missing file')
                        if artifact.suffix == '.json':
                            json.loads(artifact.read_text())
                    except (OSError, ValueError):
                        missing.append(name)
                if not missing:
                    finding = 'experiment completed; required artifacts exist'
                elif state['dead_polls'] >= config['death_polls']:
                    finding = f'incomplete terminal result; missing or invalid artifacts: {missing}'
            elif state['status_errors'] >= config['error_polls']:
                finding = f'persistent unreadable status: {status_error}'
            elif state['dead_polls'] >= config['death_polls']:
                finding = 'controller exited without a successful terminal result'
            if finding:
                event_id = hashlib.sha256(json.dumps(config, sort_keys=True).encode()).hexdigest()[:24]
                state['pending'] = {
                    'event_id': event_id, 'finding': finding, 'status': status,
                    'status_error': status_error,
                }
            atomic_json(state_path, state)
            if not finding:
                return 'running' if live else 'confirming-exit'
        pending = state['pending']
        message = (f"Experiment wake event {pending['event_id']} (at-least-once delivery; "
                   f"deduplicate this event ID). Run root: {config['root']}. "
                   f"Finding: {pending['finding']}. Exact observed status.json: "
                   f"{json.dumps(pending['status'], sort_keys=True)}. "
                   f"Status read error: {pending['status_error']}. "
                   'Review the actual results or failure and continue the authorized work. '
                   'Recheck current status before acting, respect existing user authorization, '
                   'and avoid duplicate starts if another controller is still running.')
        # Persist intent before the external side effect; a crash may repeat this ID.
        state['retry_at'] = timestamp + 60
        atomic_json(state_path, state)
        try:
            receipt = queue(config['thread'], message)
        except (OSError, subprocess.SubprocessError) as error:
            state['queue_error'] = f'{type(error).__name__}: {error}'
            atomic_json(state_path, state)
            return 'queue-retry'
        state['delivered'] = {'event_id': pending['event_id'], 'at': timestamp,
                              'queue_receipt': receipt}
        state.pop('queue_error', None)
        atomic_json(state_path, state)
        atomic_json(directory / 'delivered.json', state['delivered'])
        return 'delivered'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    setup = commands.add_parser('arm')
    setup.add_argument('--root', required=True, type=Path)
    setup.add_argument('--thread', required=True)
    setup.add_argument('--controller-pid', type=int)
    setup.add_argument('--success-phase', action='append', required=True)
    setup.add_argument('--require-artifact', action='append', default=[])
    setup.add_argument('--poll-seconds', type=float, default=30)
    setup.add_argument('--error-polls', type=int, default=3)
    setup.add_argument('--death-polls', type=int, default=2)
    for command in ('check', 'run'):
        commands.add_parser(command).add_argument('--root', required=True, type=Path)
    args = parser.parse_args()
    if args.command == 'arm':
        arm(args.root, args.thread, args.success_phase, args.require_artifact,
            args.controller_pid, args.poll_seconds, args.error_polls, args.death_polls)
        print('armed')
    else:
        print(check(args.root))


if __name__ == '__main__':
    main()
