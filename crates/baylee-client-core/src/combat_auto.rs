//! Finish the remaining shares of one explicitly approved damage division.
use crate::board::keyword_bits;
use crate::decision::resource_player;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::{NumberPrompt, Pending, PlayerAction};
use baylee_view::{PlayerView, Step};

/// A one-shot instruction; other creatures and later damage steps stay manual.
pub struct CombatAuto {
    source: ObjectId,
    turn: u32,
    step: Step,
    next: u8,
    of: u8,
    sent_at: Option<u64>,
}

impl CombatAuto {
    /// Approve only the currently offered division, including its current share.
    #[must_use]
    pub fn begin(view: &PlayerView, pending: &Pending) -> Option<Self> {
        let Pending::ChooseNumber {
            player,
            reason: NumberPrompt::CombatDamage {
                source, index, of, ..
            },
            ..
        } = *pending
        else {
            return None;
        };
        (player == resource_player(view)).then_some(Self {
            source,
            turn: view.turn,
            step: view.step,
            next: index,
            of,
            sent_at: None,
        })
    }

    /// Whether this is still the next share of the approved division.
    #[must_use]
    pub fn accepts(&self, view: &PlayerView, pending: &Pending) -> bool {
        view.turn == self.turn
            && view.step == self.step
            && matches!(pending,
            Pending::ChooseNumber {
                player,
                reason: NumberPrompt::CombatDamage { source, index, of, .. }, ..
            } if *player == resource_player(view) && *source == self.source
                && (*index == self.next || (self.sent_at == Some(view.seq)
                    && index.saturating_add(1) == self.next)) && *of == self.of)
    }

    /// Assign lethal damage in recipient order, accounting for marked damage
    /// and deathtouch. The engine gives the final recipient the remainder.
    /// When banding lets us assign damage to our own creatures, concentrate
    /// it on the current one instead of needlessly spreading it among them.
    pub fn answer(&mut self, view: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
        if self.sent_at == Some(view.seq) || !self.accepts(view, pending) {
            return None;
        }
        let Pending::ChooseNumber {
            min,
            max,
            reason: NumberPrompt::CombatDamage { recipient, .. },
            ..
        } = *pending
        else {
            return None;
        };
        let target = view.object(recipient)?;
        let touch = view
            .object(self.source)
            .is_some_and(|o| o.keywords & keyword_bits::DEATHTOUCH != 0);
        let lethal = if touch {
            1
        } else {
            u32::try_from(i32::from(target.toughness.unwrap_or(0)) - i32::from(target.damage))
                .unwrap_or(0)
        };
        let amount = if target.controller == resource_player(view) {
            max
        } else {
            lethal.clamp(min, max)
        };
        self.sent_at = Some(view.seq);
        self.next = self.next.saturating_add(1);
        Some(PlayerAction::ChooseNumber(amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, token};

    fn pending(index: u8, left: u32) -> Pending {
        pending_of(0, index, left)
    }

    /// The share `index` of the attacker's division, asked of `player`.
    fn pending_of(player: u8, index: u8, left: u32) -> Pending {
        Pending::ChooseNumber {
            player: baylee_core::ids::PlayerId::new(player),
            min: 0,
            max: left,
            reason: NumberPrompt::CombatDamage {
                source: ObjectId::new(1, 0),
                recipient: ObjectId::new(2 + u32::from(index), 0),
                index,
                of: 4,
                left,
            },
        }
    }

    #[test]
    fn remaining_shares_preserve_manual_work_and_stop_at_another_division() {
        let mut view = ViewBuilder::new(2)
            .with_battlefield(0, vec![token(1, 0, "Attacker", 8, 8)])
            .with_battlefield(
                1,
                vec![
                    token(2, 1, "First", 2, 2),
                    token(3, 1, "Second", 3, 3),
                    token(4, 1, "Third", 4, 4),
                ],
            )
            .build();
        view.battlefield
            .iter_mut()
            .find(|o| o.id == ObjectId::new(3, 0))
            .unwrap()
            .damage = 1;
        let p = pending(1, 5); // three points were assigned manually already
        let mut auto = CombatAuto::begin(&view, &p).unwrap();
        assert_eq!(auto.answer(&view, &p), Some(PlayerAction::ChooseNumber(2)));
        assert_eq!(auto.answer(&view, &p), None, "no duplicate send");
        view.seq += 1;
        assert_eq!(
            auto.answer(&view, &pending(2, 3)),
            Some(PlayerAction::ChooseNumber(3))
        );
        assert!(
            !auto.accepts(&view, &pending(0, 8)),
            "never start the division over"
        );
        assert!(!auto.accepts(
            &view,
            &Pending::ChooseNumber {
                player: view.seat,
                min: 0,
                max: 8,
                reason: NumberPrompt::X,
            }
        ));
    }

    #[test]
    fn deathtouch_bounds_and_owned_recipients_are_respected() {
        let mut source = token(1, 0, "Toucher", 5, 5);
        source.keywords |= keyword_bits::DEATHTOUCH;
        let mut view = ViewBuilder::new(2)
            .with_battlefield(0, vec![source])
            .with_battlefield(1, vec![token(2, 1, "Blocker", 6, 6)])
            .build();
        for (left, expected) in [(5, 1), (0, 0)] {
            let p = pending(0, left);
            assert_eq!(
                CombatAuto::begin(&view, &p).unwrap().answer(&view, &p),
                Some(PlayerAction::ChooseNumber(expected))
            );
        }
        view.battlefield[1].controller = view.seat;
        let p = pending(0, 5);
        let mut auto = CombatAuto::begin(&view, &p).unwrap();
        assert_eq!(auto.answer(&view, &p), Some(PlayerAction::ChooseNumber(5)));
        view.turn += 1;
        assert!(!auto.accepts(&view, &pending(1, 0)));
    }

    /// Mindslaver (CR 723.5): the controlled player's attacker divides its
    /// damage and this seat answers for it. The shortcut finishes that
    /// division, lethal to this seat's own blockers, and the controlled
    /// player's own creatures (banding) are "ours" and take the rest.
    #[test]
    fn a_controlled_attackers_division_is_this_seats_to_finish() {
        let mut view = ViewBuilder::new(2)
            .controlling(1, vec![])
            .with_battlefield(1, vec![token(1, 1, "Attacker", 8, 8)])
            .with_battlefield(0, vec![token(2, 0, "Blocker", 2, 2)])
            .build();
        assert!(
            CombatAuto::begin(&view, &pending_of(0, 0, 8)).is_none(),
            "this seat's own question is not the one being asked"
        );
        let p = pending_of(1, 0, 8);
        let mut auto = CombatAuto::begin(&view, &p).expect("the controlled division");
        assert_eq!(
            auto.answer(&view, &p),
            Some(PlayerAction::ChooseNumber(2)),
            "lethal to the blocker that is not the controlled player's"
        );
        view.battlefield[1].controller = baylee_core::ids::PlayerId::new(1);
        let p = pending_of(1, 0, 8);
        let mut auto = CombatAuto::begin(&view, &p).expect("the controlled division");
        assert_eq!(
            auto.answer(&view, &p),
            Some(PlayerAction::ChooseNumber(8)),
            "the controlled player's own creature takes it all"
        );
    }
}
