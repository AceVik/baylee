//! `cards/enchantments/mv_2/goblin_festival.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn goblin_festival() -> CardIndex {
    card_index("de21b90f-a268-46d6-b977-d5b0856176f3")
}

/// Goblin Festival (de21b90f-a268-46d6-b977-d5b0856176f3) — "{2}: This
/// enchantment deals 1 damage to any target. Flip a coin. If you lose the
/// flip, choose one of your opponents. That player gains control of this
/// enchantment."
///
/// One mana is not enough to activate; with {2}, it deals 1 damage to the
/// target, keeps control on a won flip, and gives control to the chosen
/// opponent on a lost flip.
#[test]
fn goblin_festival_requires_two_mana_deals_one_damage_and_transfers_control_on_lost_flip() {
    let mut e_short = lk::game(SEED, &[goblin_festival()], &[], &[], &[]);
    lk::float(&mut e_short, lk::P0, &[(ManaColor::Red, 1)]);
    assert!(
        !lk::offered(&e_short, goblin_festival(), lk::P0, 0),
        "one mana is one short of {{2}}"
    );

    let mut saw_win = false;
    let mut saw_loss = false;
    for seed in 1..=30 {
        let mut e = lk::game(seed, &[goblin_festival()], &[], &[], &[]);
        lk::float(&mut e, lk::P0, &[(ManaColor::Red, 2)]);
        assert!(lk::offered(&e, goblin_festival(), lk::P0, 0));
        lk::use_ability(&mut e, lk::P0, goblin_festival(), 0, &[], &[lk::P1]);
        assert_eq!(lk::life(&e, lk::P1), 19, "1 damage dealt to target player");
        if on_battlefield(&e, lk::P0, goblin_festival()).is_some() {
            assert!(on_battlefield(&e, lk::P1, goblin_festival()).is_none());
            saw_win = true;
        } else {
            assert!(
                on_battlefield(&e, lk::P1, goblin_festival()).is_some(),
                "lost flip transfers control to opponent"
            );
            saw_loss = true;
        }
        if saw_win && saw_loss {
            break;
        }
    }
    assert!(saw_win, "a won flip keeps control with P0");
    assert!(saw_loss, "a lost flip transfers control to P1");
}

/// Goblin Festival — CR 400.7: if the enchantment leaves the battlefield and
/// re-enters before its activated ability resolves, the new battlefield
/// incarnation is a different object and is not handed over on a lost flip.
#[test]
fn goblin_festival_new_incarnation_is_not_given_away_when_flip_loses() {
    for seed in 1..=20 {
        let mut e = lk::game(seed, &[goblin_festival()], &[], &[], &[]);
        let festival = lk::obj(&e, lk::P0, goblin_festival());
        let old_version = e.state().object(festival).unwrap().version;

        lk::float(&mut e, lk::P0, &[(ManaColor::Red, 2)]);
        activate(&mut e, lk::P0, goblin_festival(), 0);
        lk::aim(&mut e, &[], &[lk::P1]);

        // Blink Goblin Festival (Battlefield -> Exile -> Battlefield) while its
        // activated ability is still on the stack, creating a new incarnation.
        {
            let state = e.dev_state_mut(lk::P0).unwrap();
            state
                .move_object(
                    festival,
                    ZoneLocation::Exile(lk::P0),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
            state
                .move_object(
                    festival,
                    ZoneLocation::Battlefield,
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        }
        e.refresh_offer();
        let new_version = e.state().object(festival).unwrap().version;
        assert_ne!(new_version, old_version, "new battlefield incarnation");

        // Resolve the Festival ability; P1 takes 1 damage and the new incarnation
        // remains under P0's control regardless of the coin flip.
        lk::settle(&mut e);
        assert_eq!(lk::life(&e, lk::P1), 19);
        assert!(
            on_battlefield(&e, lk::P0, goblin_festival()).is_some(),
            "new incarnation stays under P0's control on seed {seed}"
        );
        assert!(on_battlefield(&e, lk::P1, goblin_festival()).is_none());
    }
}
