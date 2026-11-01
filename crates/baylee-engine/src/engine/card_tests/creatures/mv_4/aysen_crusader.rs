//! Aysen Crusader — `{2}{W}{W}` 2+*/2+* Creature — Human Knight (Homelands)
//!
//! Aysen Crusader's power and toughness are each equal to 2 plus the number of
//! Soldiers and Warriors you control.

#[allow(clippy::wildcard_imports)]
use super::*;

fn aysen_crusader() -> CardIndex {
    card_index("3ebbe05c-7cc4-4845-87e5-ab93d7ac9623")
}

fn benalish_hero() -> CardIndex {
    card_index("4c81cfb7-8765-4e28-ae33-4287fa9a86cc")
}

#[test]
fn aysen_crusader_pt_equals_two_plus_soldiers_and_warriors_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[aysen_crusader(), benalish_hero()])
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                benalish_hero(), // Soldier you control (#1)
                grizzly_bears(), // Bear (not a Soldier or Warrior)
            ],
        )
        .battlefield(1, &[benalish_hero()]) // opponent's Soldier (not counted)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let crusader_hand = in_hand(&engine, p0, aysen_crusader()).unwrap();
    let hero_hand = in_hand(&engine, p0, benalish_hero()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: crusader_hand,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let crusader = on_battlefield(&engine, p0, aysen_crusader()).unwrap();
    // Crusader itself is a Human Knight (not Soldier/Warrior); Benalish Hero is a Soldier -> 2 + 1 = 3/3.
    assert_eq!(pt(&engine, crusader), (3, 3));

    // Cast second Benalish Hero ({W}, Human Soldier) -> 2 + 2 = 4/4.
    engine
        .apply(p0, PlayerAction::CastSpell { card: hero_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, crusader), (4, 4));
}

/// Kindred permanents have creature subtypes without being creatures.
/// A permanent with both named subtypes still contributes only one.
#[test]
fn aysen_crusader_counts_noncreature_kindred_soldiers_and_warriors_once_each() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::{generated::subtypes, types::SubtypeSet};
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), wild_elephant(), gray_ogre()],
        &[grizzly_bears()],
        &[aysen_crusader()],
        &[],
    );
    let soldier = lk::obj(&e, p0, grizzly_bears());
    let warrior = lk::obj(&e, p0, wild_elephant());
    let both = lk::obj(&e, p0, gray_ogre());
    let opposing = lk::obj(&e, p1, grizzly_bears());
    // The real pool has no Kindred fixture. Change only the controlled
    // fixture's copiable characteristics, then cast the actual subject.
    let state = e.dev_state_mut(p0).unwrap();
    for (object, subtypes) in [
        (
            soldier,
            SubtypeSet::from_slice(&[subtypes::creature::SOLDIER]),
        ),
        (
            warrior,
            SubtypeSet::from_slice(&[subtypes::creature::WARRIOR]),
        ),
        (
            both,
            SubtypeSet::from_slice(&[subtypes::creature::SOLDIER, subtypes::creature::WARRIOR]),
        ),
        (
            opposing,
            SubtypeSet::from_slice(&[subtypes::creature::SOLDIER, subtypes::creature::WARRIOR]),
        ),
    ] {
        let base = std::sync::Arc::make_mut(&mut state.object_mut(object).unwrap().base);
        base.types = TypeSet::KINDRED.union(TypeSet::ENCHANTMENT);
        base.subtypes = subtypes;
        base.power = None;
        base.toughness = None;
    }
    state.invalidate_projections();
    state.refresh_characteristics();
    lk::float(&mut e, p0, &[(ManaColor::White, 4)]);
    lk::cast_at(&mut e, p0, aysen_crusader(), &[], &[]);
    let crusader = lk::obj(&e, p0, aysen_crusader());
    assert_eq!(
        pt(&e, crusader),
        (5, 5),
        "three owned permanents, not four subtypes or the opponent's permanent"
    );
}
