#!/usr/bin/env python3
"""Read completed Criterion runs; preserve each run's own confidence interval."""
import argparse
import json
from pathlib import Path


def read_run(run):
    rows = {}
    for path in sorted((run / 'criterion').glob('**/estimates.json')):
        if path.parent.name != run.name:
            continue
        meta = json.loads((path.parent / 'benchmark.json').read_text())
        estimates = json.loads(path.read_text())
        samples = json.loads((path.parent / 'sample.json').read_text())
        rows[meta['full_id']] = {
            'median_ns': estimates['median']['point_estimate'],
            'median_ci': estimates['median']['confidence_interval'],
            'mean_ns': estimates['mean']['point_estimate'],
            'samples': len(samples['times']),
            'raw_estimates': str(path.relative_to(run.parent.parent)),
        }
    if not rows:
        raise ValueError(f'No saved measurements for {run.name}')
    return rows


def compare(runs, prefix):
    a1, b, a2 = [runs[prefix + suffix] for suffix in ('-A1', '-B', '-A2')]
    assert a1.keys() == b.keys() == a2.keys(), prefix
    result = []
    for name in sorted(a1):
        x, y, z = [r[name]['median_ns'] for r in (a1, b, a2)]
        result.append({
            'benchmark': name,
            'A1_ns': x, 'B_ns': y, 'A2_ns': z,
            'B_vs_A1_percent': (y / x - 1) * 100,
            'B_vs_A2_percent': (y / z - 1) * 100,
            'B_vs_A_midpoint_percent': (y / ((x + z) / 2) - 1) * 100,
            'A_return_drift_percent': (z / x - 1) * 100,
            'B_outside_A_envelope': y < min(x, z) or y > max(x, z),
        })
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    runs = {p.name: read_run(p) for p in sorted((args.root / 'runs').iterdir())
            if p.is_dir() and (p / 'load-after.txt').exists()}
    prefixes = sorted({label[:-3] for label in runs if label.endswith('-A1')})
    groups = {p: compare(runs, p) for p in prefixes
              if all(p + suffix in runs for suffix in ('-B', '-A2'))}
    report = {'method': 'Independent Criterion median ns/op and confidence intervals; B versus arithmetic midpoint of bracketing A medians; no sample pooling or averaged intervals.',
              'runs': runs, 'comparisons': groups}
    if args.output:
        args.output.write_text(json.dumps(report, indent=2) + '\n')
    for prefix, rows in groups.items():
        print(prefix)
        for row in rows:
            name = row['benchmark']
            print(f"  {name}: {row['A1_ns']:.3f} / {row['B_ns']:.3f} / {row['A2_ns']:.3f} ns; "
                  f"B/mid {row['B_vs_A_midpoint_percent']:+.2f}%; A drift {row['A_return_drift_percent']:+.2f}%")


if __name__ == '__main__':
    main()
