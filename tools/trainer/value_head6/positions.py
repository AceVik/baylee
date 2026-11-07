"""Which frames of a dataset are positions a value head is scored on.

A position is the K=0 observation a seat holds when it next decides: `ov` in
`eval4 --value-log`. Every other K=0 frame only advances the recurrent memory,
and a decision row never does, so decision rows are read for their headers and
never reach the trunk.
"""
import numpy as np
import dataset4 as D
import model4 as M

NET, HOUSE, FORCED, EXTERNAL = 0, 1, 2, 4
PRIORITY = 0
DECISION, TERMINAL = 1, 2
STEP_FIRST_MAIN, STEP_SECOND_MAIN = 4, 11
ACTIVE_IS_SELF, STACK, CASTING, OWED_MANA, OWED_DAMAGE = 15, 23, 29, 31, 32
SEAT_LIFE, SEAT_HAND, SEAT_POOL = 11, 17, 33
ENTITY_BATTLEFIELD, ENTITY_CONTROLLED = 0, 12


class PositionFrames(D.TrajectoryDataset4):
    """A dataset's K=0 stream, per chosen seat, with each frame's position mark."""

    def __init__(self, data_dir, split, unroll_len, encoder_version, seats):
        super().__init__(data_dir, split=split, unroll_len=unroll_len, encoder_version=encoder_version)
        if seats not in ("net", "all"):
            raise ValueError("seats is net or all")
        self.source = np.full(len(self.store.index), FORCED, dtype=np.int8)
        self.pending = np.zeros(len(self.store.index), dtype=np.int8)
        flags = np.asarray(self.store.index["flags"])
        segments, new_trajectory, trajectories = [], [], []
        for trajectory in self.trajectories:
            rows = np.concatenate([self.segments[segment] for segment in trajectory])
            decisions = (flags[rows] & DECISION) != 0
            frames = rows[~decisions]
            if not self._mark(rows, decisions, seats) or not len(frames):
                continue
            indices = []
            for start in range(0, len(frames), unroll_len):
                indices.append(len(segments))
                segments.append(frames[start:start + unroll_len])
                new_trajectory.append(start == 0)
            trajectories.append(indices)
        self.segments, self.segment_is_new_traj, self.trajectories = segments, new_trajectory, trajectories

    def _mark(self, rows, decisions, seats):
        """Give each frame the seat's next answer; False leaves the trajectory out."""
        answered = False
        for position in np.flatnonzero(decisions):
            header = D.BST4_HDR.unpack_from(self.store.read_raw(int(rows[position])))
            pending, source = header[8], header[9]
            if source == HOUSE and seats == "net":
                return False
            if source == NET:
                answered = True
            if source == FORCED or position == 0 or decisions[position - 1]:
                continue
            frame = rows[position - 1]
            self.source[frame], self.pending[frame] = source, pending
        return answered or seats == "all"

    def _encode_bst4(self, buf):
        encoded = super()._encode_bst4(buf)
        encoded["turn"] = np.array(D.BST4_HDR.unpack_from(buf)[5], dtype=np.int64)
        return encoded


def describe(host):
    """Per frame of `[lanes, unroll, ...]` tensors, what `value_line` reads off the view.

    Hand counts saturate at seven: Encoder 7 encodes them so.
    """
    seats = host["seat_feat"].numpy()
    glob = host["global_feat"].numpy()
    entities = host["entity_feat"].numpy()
    mask = host["entity_mask"].numpy()
    battlefield = (entities[..., ENTITY_BATTLEFIELD] > .5) & mask
    own = battlefield & (entities[..., ENTITY_CONTROLLED] > .5)
    active = glob[..., ACTIVE_IS_SELF] > .5
    main = (glob[..., STEP_FIRST_MAIN] > .5) | (glob[..., STEP_SECOND_MAIN] > .5)
    quiet = ((glob[..., STACK] == 0) & (glob[..., CASTING] == 0) & (glob[..., OWED_MANA] == 0)
             & (glob[..., OWED_DAMAGE] == 0) & (seats[..., 0, SEAT_POOL] == 0))
    return dict(active=active, main_quiet=active & main & quiet,
                life=np.rint(seats[..., 0, SEAT_LIFE] * 40), opp_life=np.rint(seats[..., 1, SEAT_LIFE] * 40),
                hand=np.rint(seats[..., 0, SEAT_HAND] * 7), opp_hand=np.rint(seats[..., 1, SEAT_HAND] * 7),
                board=own.sum(-1), opp_board=battlefield.sum(-1) - own.sum(-1),
                lost=(seats[..., :2, M.SEAT_FEAT_LOST] > .5).any(-1))
