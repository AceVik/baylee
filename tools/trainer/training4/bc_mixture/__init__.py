"""Opt-in BC-only mixture: fixed half lanes, source-local stores, whole-game split."""
from itertools import zip_longest
import numpy as np
from torch.utils.data import Dataset, Sampler
from sampler4 import StatefulBatchSampler4
from .metadata import digest,game_metadata,reject_overlap


class SourceView:
    def __init__(self, child, games):
        self.child=child;self.segments=[];self.trajectories=[]
        for trajectory in child.trajectories:
            row=child.segments[trajectory[0]][0]
            if int(child.store.index['game'][row]) not in games:continue
            first=len(self.segments);self.segments.extend(trajectory)
            self.trajectories.append(list(range(first,len(self.segments))))
        if not self.segments:raise ValueError('BC mixture source partition is empty')


class BCMixture(Dataset):
    """The primary old source always uses stable table-hash half0 in both arms.

    Same source path as replay => complementary half1 (old-only control).
    A different replay path => its entire training split (corrective arm).
    No frame payload/ID is rewritten or put in a shared TrajectoryStore.
    """
    def __init__(self, anchor, replay, validation, batch_size):
        versions = {getattr(child, 'encoder_version', 7) for child in (anchor, replay, validation)}
        if len(versions) != 1:raise ValueError('BC mixture encoder versions differ')
        self.encoder_version = versions.pop()
        if batch_size<2 or batch_size%2:raise ValueError('BC mixture needs positive even batch size')
        if anchor.structure.digest!=replay.structure.digest or anchor.structure.digest!=validation.structure.digest:
            raise ValueError('BC mixture public catalogs differ')
        if anchor.unroll_len!=replay.unroll_len:raise ValueError('BC mixture unroll lengths differ')
        a,ar=game_metadata(anchor,'train');v,vr=game_metadata(validation,'val')
        same=anchor.data_dir.resolve()==replay.data_dir.resolve()
        b,br=(a,ar) if same else game_metadata(replay,'train')
        left={g:x for g,x in a.items() if int(x['table'],16)%2==0}
        right={g:x for g,x in b.items() if not same or int(x['table'],16)%2==1}
        reject_overlap(left,right);reject_overlap(a,v);reject_overlap(b,v)
        self.sources=(SourceView(anchor,left),SourceView(replay,right))
        self.offset=len(self.sources[0].segments);self.structure=anchor.structure
        self.contract=dict(version=1,kind='bc_only_half_lane_mixture',ppo_eligible=False,
            lane_counts=[batch_size//2,batch_size//2],proportion_scope='Half physical persistent lanes, not equal labeled choices/loss weights; exhausted tails may be zero padding',
            anchor_partition='sha256(namespace+table_seed)%2==0',
            replay_partition='complementary_half1' if same else 'all',
            sources=[dict(path=str(c.data_dir.resolve()),fingerprint=c.fingerprint,
                          games=len(g),selection_sha256=digest(sorted((k,x) for k,x in g.items())))
                     for c,g in [(anchor,left),(replay,right)]],
            source_metadata_sha256={**ar,**br,**vr},reset='Child trajectory starts and exhausted lanes reset H; source lane assignments never change.')
        self.fingerprint=digest(self.contract)
        self.trajectories=self.sources[0].trajectories+[[i+self.offset for i in t] for t in self.sources[1].trajectories]

    def __len__(self):return sum(len(s.segments) for s in self.sources)

    def __getitem__(self,index):
        if index==-1:
            item=dict(self.sources[0].child[-1]);source=-1
        else:
            source=int(index>=self.offset);local=index-self.offset if source else index
            view=self.sources[source];item=dict(view.child[view.segments[local]])
        item['segment_index']=np.array(index,dtype=np.int64)
        item['bc_source']=np.array(source,dtype=np.int64)
        return item


class BCMixtureSampler(Sampler):
    VERSION=1
    def __init__(self,dataset,batch_size,seed,epoch_batches=0):
        if type(epoch_batches) is not int or epoch_batches<0:
            raise ValueError("BC epoch batches must be a nonnegative integer")
        self.epoch_batches=epoch_batches
        self.dataset=dataset;self.seed=seed;self.epoch=0;self.start_batch=0
        self.children=[StatefulBatchSampler4(v,batch_size//2,shuffle=True,seed=seed+i)
                       for i,v in enumerate(dataset.sources)]
        self.half=batch_size//2
        self.full_lane_prefix_batches=[]
        for child in self.children:
            rows=child._build_batches(advance_epoch=False)
            self.full_lane_prefix_batches.append(next((i for i,row in enumerate(rows)
                if len(row)<self.half or any(index<0 for index in row)),len(rows)))
        if epoch_batches:
            from .cycling import CYCLE_EPOCH_STRIDE
            _,report=self._cycled_batches()
            dataset.contract={**dataset.contract,'cycling':dict(version=1,epoch_batches=epoch_batches,
                seed=seed,cycle_epoch_stride=CYCLE_EPOCH_STRIDE,
                ending='Complete first source passes; final repeated pass may end at budget mid-trajectory',
                epoch0_sources=report)}
            dataset.fingerprint=digest(dataset.contract)

    def _cycled_batches(self):
        from .cycling import cycle_source
        streams=[];reports=[]
        for child in self.children:
            rows,report=cycle_source(child,self.half,self.epoch,self.epoch_batches)
            streams.append(rows);reports.append(report)
        batches=[a+[i+self.dataset.offset if i>=0 else -1 for i in b]
                 for a,b in zip(*streams)]
        return batches,reports

    def schedule_report(self):
        """Current epoch source exposure, including retained per-pass tail padding."""
        if not self.epoch_batches:return None
        return self._cycled_batches()[1]

    def _batches(self,advance):
        if self.epoch_batches:
            batches,_=self._cycled_batches()
            if advance:self.epoch+=1
            return batches
        streams=[]
        for child in self.children:
            child.epoch=self.epoch
            rows=child._build_batches(advance_epoch=False)
            # Existing sampler uses fewer lanes for tiny sources; keep fixed positions.
            streams.append([row+[-1]*(self.half-len(row)) for row in rows])
        if advance:self.epoch+=1
        empty=[-1]*self.half
        return [a+[i+self.dataset.offset if i>=0 else -1 for i in b]
                for a,b in zip_longest(*streams,fillvalue=empty)]

    def __iter__(self):yield from self._batches(True)[self.start_batch:]
    def __len__(self):return self.epoch_batches or len(self._batches(False))


def check_resume(saved, contract):
    if saved.get('bc_mixture')!=contract:raise ValueError('Resume BC mixture contract differs')


def validate_budget(full_lane_prefix_batches, updates, epochs):
    """Optional matched-experiment gate; general full-epoch/tail training is allowed.

    Supply both arms' concatenated epoch0 source prefixes to establish a common
    positive update window without exhausted physical lanes or epoch repeats.
    """
    if epochs!=1 or updates<1 or not full_lane_prefix_batches or updates>min(full_lane_prefix_batches):
        raise ValueError('Matched BC budget requires one epoch and updates within every full-lane prefix')
