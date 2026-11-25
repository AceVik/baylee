//! Locally authorize engine projections only for a released commit or the
//! specific seat that requested a snapshot after historical recovery.
use super::{Head, Msg, VecDeque};

#[derive(Default)]
pub(super) struct Releases {
    current: Option<Head>,
    visible: VecDeque<(Head, u8)>,
}

impl Releases {
    pub(super) fn order(&mut self, order: &Msg) {
        match order {
            Msg::RankedEngineCommit(commit) => {
                let Ok(chain) = commit.chain.as_slice().try_into() else {
                    return;
                };
                let head = Head {
                    position: commit.position,
                    chain,
                };
                self.current = Some(head);
                if commit.publish {
                    self.allow(head, 3);
                }
            }
            Msg::RankedEngineSnapshot(snap) if snap.seat < 2 => {
                if let Some(head) = self.current.filter(|h| h.position == snap.position) {
                    self.allow(head, 1 << snap.seat);
                }
            }
            _ => {}
        }
    }
    fn allow(&mut self, head: Head, mask: u8) {
        if let Some((_, seats)) = self.visible.back_mut().filter(|(last, _)| *last == head) {
            *seats |= mask;
        } else {
            if self.visible.len() == 8 {
                self.visible.pop_front();
            }
            self.visible.push_back((head, mask));
        }
    }
    pub(super) fn permits(&self, seat: u32, position: u64, chain: &[u8]) -> bool {
        seat < 2
            && self.visible.iter().any(|(h, seats)| {
                h.position == position && h.chain.as_slice() == chain && seats & (1 << seat) != 0
            })
    }
}

#[cfg(test)]
mod tests {
    use super::super::v1;
    use super::*;

    #[test]
    fn silent_replay_can_only_emit_a_requested_seats_current_snapshot() {
        let mut released = Releases::default();
        let commit = |position: u64, byte: u8, publish: bool| {
            Msg::RankedEngineCommit(v1::RankedEngineCommit {
                position,
                chain: vec![byte; 32],
                publish,
                ..Default::default()
            })
        };
        released.order(&commit(0, 11, false));
        released.order(&commit(1, 12, false));
        released.order(&Msg::RankedEngineSnapshot(v1::RankedEngineSnapshot {
            position: 0,
            seat: 1,
            ..Default::default()
        }));
        released.order(&Msg::RankedEngineSnapshot(v1::RankedEngineSnapshot {
            position: 1,
            seat: 1,
            ..Default::default()
        }));
        assert!(!released.permits(1, 0, &[11; 32]));
        assert!(!released.permits(0, 1, &[12; 32]));
        assert!(released.permits(1, 1, &[12; 32]));
        released.order(&commit(2, 13, true));
        assert!(released.permits(0, 2, &[13; 32]) && released.permits(1, 2, &[13; 32]));
        assert!(!released.permits(2, 2, &[13; 32]) && !released.permits(0, 2, &[44; 32]));
    }
}
