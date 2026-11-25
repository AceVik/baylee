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

    fn commit(position: u64, publish: bool) -> Msg {
        Msg::RankedEngineCommit(v1::RankedEngineCommit {
            position,
            chain: vec![u8::try_from(position).unwrap(); 32],
            publish,
            ..Default::default()
        })
    }

    fn snapshot(position: u64, seat: u32) -> Msg {
        Msg::RankedEngineSnapshot(v1::RankedEngineSnapshot {
            position,
            seat,
            ..Default::default()
        })
    }

    fn seen(released: &Releases, position: u64) -> [bool; 2] {
        let chain = [u8::try_from(position).unwrap(); 32];
        [0, 1].map(|seat| released.permits(seat, position, &chain))
    }

    #[test]
    fn nothing_is_released_before_an_order_says_so() {
        let mut released = Releases::default();
        assert_eq!(seen(&released, 0), [false, false]);
        released.order(&snapshot(0, 0));
        assert_eq!(seen(&released, 0), [false, false], "a snapshot of nothing");
        released.order(&commit(1, false));
        assert_eq!(seen(&released, 1), [false, false], "a silent commit");
    }

    #[test]
    fn a_snapshot_is_the_asking_seats_and_of_no_seat_the_game_lacks() {
        let mut released = Releases::default();
        released.order(&commit(1, false));
        for seat in [2, 3, 8, 32, u32::MAX] {
            released.order(&snapshot(1, seat));
            assert_eq!(seen(&released, 1), [false, false], "{seat}");
            assert!(!released.permits(seat, 1, &[1; 32]));
        }
        released.order(&snapshot(1, 0));
        assert_eq!(seen(&released, 1), [true, false]);
        released.order(&snapshot(1, 1));
        assert_eq!(seen(&released, 1), [true, true], "each its own");
        // The snapshot of a head that has moved on is not taken any more.
        released.order(&commit(2, false));
        released.order(&snapshot(1, 0));
        assert_eq!(seen(&released, 2), [false, false]);
    }

    #[test]
    fn a_projection_names_its_head_s_whole_chain() {
        let mut released = Releases::default();
        released.order(&commit(1, true));
        assert!(released.permits(0, 1, &[1; 32]));
        for chain in [&[1; 31][..], &[1; 33], &[], &[2; 32]] {
            assert!(!released.permits(0, 1, chain), "{chain:?}");
        }
        assert!(!released.permits(0, 2, &[1; 32]));
        // A commit whose chain is none is no head.
        let mut short = Releases::default();
        short.order(&Msg::RankedEngineCommit(v1::RankedEngineCommit {
            position: 1,
            chain: vec![1; 31],
            publish: true,
            ..Default::default()
        }));
        short.order(&snapshot(1, 0));
        assert!(!short.permits(0, 1, &[1; 31]) && !short.permits(0, 1, &[1; 32]));
    }

    #[test]
    fn only_the_last_eight_heads_stay_released() {
        let mut released = Releases::default();
        for position in 1..=9 {
            released.order(&commit(position, true));
        }
        assert_eq!(seen(&released, 1), [false, false]);
        for position in 2..=9 {
            assert_eq!(seen(&released, position), [true, true], "{position}");
        }
        // The same head again is not another one.
        released.order(&snapshot(9, 0));
        assert_eq!(seen(&released, 2), [true, true]);
    }
}
