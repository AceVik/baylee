"""Rebuild the shipped mono PCM bank from pinned CC0 WAVs. Requires numpy, ffmpeg.
Run from repository root. Raw downloads stay under /tmp; manifest records hashes.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import urllib.parse
import urllib.request
import numpy as np

ROOT = Path('crates/baylee-client-core/assets/orchestra')
RAW = Path('/tmp/baylee-orchestra-raw')
ROOT.mkdir(parents=True, exist_ok=True)
RAW.mkdir(exist_ok=True)
# MIDI pitches verified spectrally: this library mixes octave naming conventions.
PITCH = {'violins':72, 'violas':72, 'cellos':60, 'spiccato1':72,
         'spiccato2':72, 'harp-low':48, 'harp-mid':62, 'harp-high':72,
         'flute':72, 'horn':60, 'horn-forte':60, 'oboe':74,
         'timpani':42, 'snare1':60, 'snare2':60, 'cymbal':60}
items = json.loads(Path('art/music/samples.json').read_text())
for item in items:
    name = item['name']
    raw = RAW / (name + '.wav')
    if not raw.exists():
        url = 'https://raw.githubusercontent.com/sgossner/VSCO-2-CE/' + item['revision'] + '/' + urllib.parse.quote(item['path'])
        raw.write_bytes(urllib.request.urlopen(url, timeout=60).read())
    assert hashlib.sha256(raw.read_bytes()).hexdigest() == item['sha256']
    data = subprocess.check_output(['ffmpeg','-v','error','-i',str(raw),'-ac','1','-ar','22050','-f','f32le','-'])
    samples = np.frombuffer(data,dtype='<f4').copy()
    samples -= np.mean(samples)
    # Retain the recorded attack, with only silent leading frames removed.
    significant = np.flatnonzero(np.abs(samples) > max(abs(samples)) * .012)
    start = max(0, int(significant[0]) - 220)
    samples = samples[start:start + 22050 * 8]
    samples *= .72 / max(abs(samples))
    n = min(1102,len(samples)//4)
    samples[-n:] *= np.linspace(1,0,n)
    samples[:110] *= np.linspace(0,1,110)
    pcm = np.round(samples * 32767).astype('<i2').tobytes()
    (ROOT/(name+'.pcm')).write_bytes(pcm)
    item.update(midi=PITCH[name],frames=len(samples),rate=22050,pcm_sha256=hashlib.sha256(pcm).hexdigest())
Path('art/music/samples.json').write_text(json.dumps(items,indent=2)+'\n')
