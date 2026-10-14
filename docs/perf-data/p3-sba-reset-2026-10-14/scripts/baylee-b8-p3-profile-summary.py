#!/usr/bin/env python3
"""Attribute sample call-tree nodes without counting nested inclusive samples twice."""
import json
import re
from collections import defaultdict
from pathlib import Path

root=Path('/tmp/baylee-sba-benchmark')
result={}
for variant in ('A','B'):
    counts=defaultdict(int)
    examples={}
    stack=[]
    for line_number,line in enumerate((root/'profiles'/variant/'sample.txt').read_text().splitlines(),1):
        match=re.match(r'^([ |+!:]*)(\d+) (.*)$',line)
        if not match:
            continue
        depth=len(match[1])
        count=int(match[2])
        symbol=match[3]
        while stack and stack[-1][0]>=depth:
            stack.pop()
        context=[item[1] for item in stack]
        stage='other'
        for ancestor in context:
            if '12iter_batched' in ancestor:
                if re.search(r'bencher.rs:(247|262)$',ancestor): stage='setup'
                elif re.search(r'bencher.rs:(250|266)$',ancestor): stage='timed'
                else: stage='other'
        slot_arc=lambda text: '5arena4Slot' in text and '8make_mut' in text
        if slot_arc(symbol) and not any(slot_arc(s) for s in context):
            counts[stage+'_arena_make_mut']+=count
            if any('11run_machine' in s and s.endswith('progress.rs:534') for s in context):
                key=stage+'_sba_arena_make_mut'
                counts[key]+=count
                examples.setdefault(key,{'line':line_number,'sba_caller':[s for s in context if '11run_machine' in s and s.endswith('progress.rs:534')][-1],'node':symbol})
        if '12iter_batched' in symbol:
            if re.search(r'bencher.rs:(247|262)$',symbol): counts['setup_inclusive']+=count
            if re.search(r'bencher.rs:(250|266)$',symbol): counts['timed_inclusive']+=count
        stack.append((depth,symbol))
    result[variant]={'sample_counts':dict(counts),'example_nodes':examples}
result['method']='Inclusive Arc<[Slot<GameObject>]>::make_mut call-tree samples, excluding nested duplicates. Classify setup at Criterion bencher.rs:247/262 and timed routine at :250/266. SBA caller is run_machine at progress.rs:534 (run_state_based_actions); inner SBA functions are inlined. Counts are statistical stack samples, not allocations, calls, or normalized work.'
(root/'profile-summary.json').write_text(json.dumps(result,indent=2)+'\n')
for variant in ('A','B'): print(variant,result[variant]['sample_counts'],result[variant]['example_nodes'])
