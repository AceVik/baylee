"""Persistent trajectory lanes for recurrent training and validation."""
from __future__ import annotations

import random
import heapq
from itertools import zip_longest
from typing import TYPE_CHECKING, Iterator
from torch.utils.data import Sampler

if TYPE_CHECKING:
    from dataset4 import TrajectoryDataset4


class StatefulBatchSampler4(Sampler[list[int]]):
    """Streams multi-segment trajectories across `batch_size` persistent lanes
    so recurrent state `h` can be carried across consecutive segments (`P0-4`).
    """

    VERSION = 2

    def __init__(
        self,
        dataset: TrajectoryDataset4,
        batch_size: int,
        shuffle: bool = True,
        seed: int = 42,
    ):
        self.dataset = dataset
        self.batch_size = max(1, batch_size)
        self.shuffle = shuffle
        self.seed = seed
        self.epoch = 0

    def _build_batches(self, advance_epoch: bool = True) -> list[list[int]]:
        trajs = [list(t) for t in self.dataset.trajectories if t]
        if not trajs:
            return []
        rng = random.Random(self.seed + self.epoch)
        if advance_epoch:
            self.epoch += 1
        if self.shuffle:
            # Assign whole trajectories to equally loaded lanes before shuffling
            # each lane. A shared FIFO can leave a long winning trajectory alone
            # for hundreds of updates, overweighting its label and wasting GPU
            # slots. Length balancing never reads outcomes or skips a segment.
            rng.shuffle(trajs)
            lanes = [[] for _ in range(min(self.batch_size, len(trajs)))]
            loads = [(0, i) for i in range(len(lanes))]
            for trajectory in sorted(trajs, key=len, reverse=True):
                length, lane = heapq.heappop(loads)
                lanes[lane].append(trajectory)
                heapq.heappush(loads, (length + len(trajectory), lane))
            streams = []
            for lane in lanes:
                rng.shuffle(lane)
                streams.append([segment for trajectory in lane for segment in trajectory])
            return [list(batch) for batch in zip_longest(*streams, fillvalue=-1)]

        # Validation keeps its existing, fixed grouping and scoring protocol.
        B = min(self.batch_size, len(trajs))
        queue_idx = B
        lanes: list[list[int]] = [trajs[i] for i in range(B)]
        cursors: list[int] = [0] * B
        batches: list[list[int]] = []

        while True:
            batch: list[int] = []
            active_any = False
            for b in range(B):
                if cursors[b] >= len(lanes[b]):
                    if queue_idx < len(trajs):
                        lanes[b] = trajs[queue_idx]
                        queue_idx += 1
                        cursors[b] = 0
                    else:
                        continue
                active_any = True
            if not active_any:
                break
            for b in range(B):
                if cursors[b] < len(lanes[b]):
                    batch.append(lanes[b][cursors[b]])
                    cursors[b] += 1
                else:
                    batch.append(-1)
            batches.append(batch)
        return batches

    def __iter__(self) -> Iterator[list[int]]:
        yield from self._build_batches()[getattr(self, "start_batch", 0):]

    def __len__(self) -> int:
        return len(self._build_batches(advance_epoch=False))

