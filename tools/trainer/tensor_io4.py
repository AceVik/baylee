"""Thin tensor-worker protocol: Rust owns locks, temporary IO and publication."""
import json
from pathlib import Path
import subprocess
from trajectory_store4 import _executable


class TensorPublication:
    def __init__(self, kind, output, **options):
        self.request=dict(op='prepare',kind=kind,out=str(Path(output).expanduser()),**options)
        self.process=None
        self.committed=False

    def _exchange(self, _block=None, **value):
        self.process.stdin.write(json.dumps(value,allow_nan=False)+'\n')
        self.process.stdin.flush()
        if _block is not None:
            self.process.stdin.buffer.write(_block)
            self.process.stdin.buffer.flush()
        line=self.process.stdout.readline()
        if not line:
            raise RuntimeError('Native tensor publication worker ended before acknowledgement')
        response=json.loads(line)
        if not response['ok']:
            error=FileExistsError if response.get('exists') else ValueError
            raise error(response['error'])
        return response['value']

    def __enter__(self):
        self.process=subprocess.Popen([_executable(),'tensor-io'],stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        try:
            self.receipt=self._exchange(**self.request)
            self.path=Path(self.receipt['temporary']) if not self.receipt['reused'] else None
            return self
        except BaseException:
            self._close()
            raise

    def write(self, block):
        self._exchange(op='write',length=len(block),_block=block)

    def commit(self, **options):
        result=self._exchange(op='commit',**options)
        self.committed=True
        return result

    def _close(self):
        if self.process is not None:
            try:
                self.process.stdin.close()
            except (BrokenPipeError,OSError):
                pass
            self.process.stdin=None
            try:
                self.process.communicate(timeout=30)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.communicate()
            self.process=None

    def __exit__(self, kind, value, traceback):
        try:
            if kind is None and not self.committed and self.process.poll() is None:
                self._exchange(op='abort')
        finally:
            self._close()
