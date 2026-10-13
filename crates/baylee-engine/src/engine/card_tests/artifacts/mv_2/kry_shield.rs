//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn kry_shield_prevents_both_damage_kinds_and_freezes_the_mana_value_bonus() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::KRY_SHIELD,
                index::GRIZZLY_BEARS,
                index::FOREST,
                index::FOREST,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    activate(&mut e, p0, index::KRY_SHIELD, 0);
    aim_at(&mut e, p0, bear);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, bear), (2, 4));
    for combat in [false, true] {
        let state = e.dev_state_mut(p0).unwrap();
        let mut work = crate::damage::DamageWork::new(
            state,
            vec![crate::damage::Assignment {
                source: bear,
                source_version: Some(state.object(bear).unwrap().version),
                recipient: crate::event::DamageTarget::Player(p1),
                amount: 3,
                is_combat: combat,
            }],
        );
        assert!(work.advance(state).is_none());
        assert_eq!(work.dealt(), 0);
    }
    reach_their_main_phase(&mut e, p1);
    assert_eq!(pt(&e, bear), (2, 2));
    let state = e.dev_state_mut(p0).unwrap();
    let mut work = crate::damage::DamageWork::new(
        state,
        vec![crate::damage::Assignment {
            source: bear,
            source_version: Some(state.object(bear).unwrap().version),
            recipient: crate::event::DamageTarget::Player(p1),
            amount: 1,
            is_combat: false,
        }],
    );
    assert!(work.advance(state).is_none());
    assert_eq!(work.dealt(), 1);
}

/// "Prevent all damage that would be dealt this turn by **target creature you
/// control**": the Shield's target is the only one that goes quiet. A second Bears
/// of mine and the opponent's creature still deal their damage in full, in combat
/// and out of it, and the opponent's creature is not on the Shield's menu.
#[test]
fn kry_shield_prevents_only_the_targeted_creatures_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::KRY_SHIELD,
                index::GRIZZLY_BEARS,
                index::GRIZZLY_BEARS,
                index::FOREST,
                index::FOREST,
            ],
        )
        .battlefield(1, &[index::LLANOWAR_ELVES])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    let bears = all_on_battlefield(&e, p0, index::GRIZZLY_BEARS);
    assert_eq!(bears.len(), 2, "two Bears of mine");
    let (shielded, bystander) = (bears[0], bears[1]);
    let theirs = on_battlefield(&e, p1, index::LLANOWAR_ELVES).unwrap();
    let shield = on_battlefield(&e, p0, index::KRY_SHIELD).unwrap();

    activate(&mut e, p0, index::KRY_SHIELD, 0);
    let options = aim_at(&mut e, p0, shielded);
    assert!(
        options.contains(&shielded) && options.contains(&bystander),
        "either creature I control may be shielded: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\": the opponent's Elves are not offered: {options:?}"
    );
    assert!(
        !options.contains(&shield),
        "the Shield is no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    pass_until(&mut e, stack_is_empty);

    for (source, label, recipient, dealt_in_full) in [
        (shielded, "the targeted Bears (control)", p1, false),
        (bystander, "my other Bears", p1, true),
        (theirs, "the opponent's Elves", p0, true),
    ] {
        for combat in [false, true] {
            let state = e.dev_state_mut(p0).unwrap();
            let mut work = crate::damage::DamageWork::new(
                state,
                vec![crate::damage::Assignment {
                    source,
                    source_version: Some(state.object(source).unwrap().version),
                    recipient: crate::event::DamageTarget::Player(recipient),
                    amount: 3,
                    is_combat: combat,
                }],
            );
            assert!(work.advance(state).is_none());
            assert_eq!(
                work.dealt(),
                if dealt_in_full { 3 } else { 0 },
                "{} damage from {label}: the Shield stops only the creature it targeted",
                if combat { "combat" } else { "noncombat" }
            );
        }
    }
    assert_eq!(
        pt(&e, bystander),
        (2, 2),
        "and only the targeted Bears got the toughness bonus"
    );
}
