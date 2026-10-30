"""Thin Torch Dataset over the persistent, strictly verified Rust tensor server.

All card lookup, feature packing, provenance and target admission live in Rust.
This adapter decodes typed buffers and pads Torch unrolls; it never constructs
ordinary outcome labels or repairs masks, picks, or semantic references.
"""
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess

import numpy as np
import torch
from .targets import OBJECTIVE

MAX_DESCRIPTOR = 16 * 1024 * 1024
MAX_PAYLOAD = 512 * 1024 * 1024
DTYPES = {'f32': np.dtype('<f4'), 'i64': np.dtype('<i8'), 'u8': np.dtype('u1')}
SCHEMA = 'failure_terminal_research_tensors_v1'


def executable():
    path = os.environ.get('BAYLEE_RESEARCH_TENSOR') or shutil.which('research_tensor4')
    if path:
        return path
    root = Path(__file__).resolve().parents[3]
    for profile in ('release', 'debug'):
        path = root / 'target' / profile / 'research_tensor4'
        if path.is_file():
            return str(path)
    raise RuntimeError('Build research_tensor4 with cargo before research PPO')


def exact_read(stream, length):
    data = bytearray(length)
    view = memoryview(data)
    offset = 0
    while offset < length:
        count = stream.readinto(view[offset:])
        if not count:
            raise ValueError('Native research tensor stream ended prematurely')
        offset += count
    return data


def decode_response(stream):
    length, = struct.unpack('<Q', exact_read(stream, 8))
    if not 0 < length <= MAX_DESCRIPTOR:
        raise ValueError('Invalid research tensor descriptor size')
    descriptor = json.loads(exact_read(stream, length))
    if descriptor.get('schema') != SCHEMA:
        raise ValueError('Unsupported native research tensor schema')
    length, = struct.unpack('<Q', exact_read(stream, 8))
    if length > MAX_PAYLOAD:
        raise ValueError('Research tensor payload exceeds Torch adapter bound')
    payload = exact_read(stream, length)
    arrays = {}
    offset = 0
    for item in descriptor['arrays']:
        name, shape = item['name'], item['shape']
        if (name in arrays or item['dtype'] not in DTYPES or not isinstance(shape, list)
                or any(type(n) is not int or n < 0 for n in shape)):
            raise ValueError('Invalid native tensor array descriptor')
        dtype = DTYPES[item['dtype']]
        count = 1
        for dim in shape:
            count *= dim
        nbytes = count * dtype.itemsize
        if item['offset'] != offset or item['nbytes'] != nbytes or offset + nbytes > length:
            raise ValueError('Native tensor array extent differs')
        # Tensor operations may write target caches; never hand them read-only views.
        array = np.frombuffer(payload, dtype=dtype, count=count, offset=offset).reshape(shape).copy()
        if item['dtype'] == 'u8':
            if not np.isin(array, (0, 1)).all():
                raise ValueError('Native boolean tensor contains non-boolean values')
            array = array.astype(np.bool_)
        arrays[name] = array
        offset += nbytes
    if offset != length:
        raise ValueError('Trailing native tensor bytes')
    if any(name in arrays for name in ('outcome', 'aux_target', 'is_terminal')):
        raise ValueError('Ordinary target fields in research tensor stream')
    return descriptor, arrays


class ResearchDataset(torch.utils.data.Dataset):
    """Single-process complete-history dataset; native server owns source checks."""
    def __init__(self, cache_dir, conversion_sha, cache_sha, pins, pins_sha, admission, unroll_len=64):
        super().__init__()
        if not 1 <= unroll_len <= 256:
            raise ValueError('Research unroll must be between 1 and 256')
        self.unroll_len = unroll_len
        self.owner_pid = os.getpid()
        self.process = None
        command = [executable(), 'serve', '--cache-dir', str(cache_dir),
                   '--expected-conversion-manifest-sha256', conversion_sha,
                   '--expected-cache-manifest-sha256', cache_sha,
                   '--pins', str(pins), '--expected-pins-sha256', pins_sha,
                   '--admission', str(admission)]
        self.verify_command = [command[0], 'verify', '--pins', str(pins),
                               '--expected-pins-sha256', pins_sha, '--admission', str(admission)]
        try:
            self.process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
            line = self.process.stdout.readline(MAX_DESCRIPTOR + 1)
            if not line.endswith(b'\n') or len(line) > MAX_DESCRIPTOR:
                raise ValueError('Missing/bounded native research handshake')
            self.metadata = json.loads(line)
            if (self.metadata.get('schema') != SCHEMA or self.metadata.get('objective') != OBJECTIVE
                    or self.metadata.get('conversion_manifest_sha256') != conversion_sha
                    or self.metadata.get('cache_manifest_sha256') != cache_sha):
                raise ValueError('Native research handshake contract differs')
            self.admission = self.metadata['admission']
            if (not isinstance(self.admission, dict)
                    or self.admission.get('research_input_admitted') is not True
                    or self.admission.get('ordinary_ppo_admitted') is not False
                    or self.admission.get('numerical_replay_required') is not True
                    or self.admission.get('objective') != OBJECTIVE
                    or self.admission.get('pins_sha256') != pins_sha):
                raise ValueError('Research tensor session is not admitted for numerical replay')
            self.fingerprint = self.metadata['content_fingerprint']
            self.trajectories, self.segments = [], []
            self.histories = self.metadata['histories']
            for history_id, history in enumerate(self.histories):
                trajectory = []
                for start in range(0, history['frames'], unroll_len):
                    trajectory.append(len(self.segments))
                    self.segments.append((history_id, start, min(unroll_len, history['frames'] - start)))
                self.trajectories.append(trajectory)
            if not self.segments:
                raise ValueError('Research tensor session contains no known-return histories')
        except BaseException:
            self.close()
            raise

    def __len__(self):
        return len(self.segments)

    def __getitem__(self, index):
        if os.getpid() != self.owner_pid:
            raise RuntimeError('Research tensor session cannot be shared across DataLoader processes')
        if index == -1:
            row = {key: np.zeros_like(value) for key, value in self[0].items()}
            row['cand_mask'][:, 0] = True
            row['pair_mask'][:, 0] = True
            row['is_new_traj'] = np.array(True)
            row['segment_index'] = np.array(-1, dtype=np.int64)
            return row
        history, start, length = self.segments[index]
        request = dict(history=history, start=start, length=length)
        self.process.stdin.write(json.dumps(request).encode() + b'\n')
        self.process.stdin.flush()
        descriptor, arrays = decode_response(self.process.stdout)
        identity = descriptor['identity']
        expected = dict(history=history, start=start, length=length,
                        attempt_id=self.histories[history]['attempt_id'],
                        episode_id=self.histories[history]['episode_id'], seat=self.histories[history]['seat'],
                        new_history=start == 0,
                        complete_history=start == 0 and length == self.histories[history]['frames'],
                        returns=self.histories[history]['returns'])
        if identity != expected:
            raise ValueError('Native research response belongs to a different history/range')
        if any(array.ndim < 1 or array.shape[0] != length for array in arrays.values()):
            raise ValueError('Native tensor time axis differs from requested unroll')
        # The only padding introduced here is a masked tail for the Torch unroll.
        if length < self.unroll_len:
            padded = {}
            for name, array in arrays.items():
                result = np.zeros((self.unroll_len, *array.shape[1:]), dtype=array.dtype)
                result[:length] = array
                padded[name] = result
            arrays = padded
            arrays['cand_mask'][length:, 0] = True
            arrays['pair_mask'][length:, 0] = True
        arrays['is_new_traj'] = np.array(start == 0)
        arrays['segment_index'] = np.array(index, dtype=np.int64)
        arrays['behavior_lp'] = arrays['behavior_logprob'].copy()
        if hasattr(self, 'ppo_cache'):
            arrays.update({name: array[index] for name, array in self.ppo_cache.items()})
        return arrays

    def close(self):
        process, self.process = self.process, None
        if process is not None:
            process.stdin.close()
            process.stdout.close()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)

    def verify(self):
        """Recheck the frozen native input receipt before checkpoint publication."""
        result = subprocess.run(self.verify_command, capture_output=True)
        if result.returncode:
            raise ValueError('Research input admission changed: ' + result.stderr.decode(errors='replace'))
