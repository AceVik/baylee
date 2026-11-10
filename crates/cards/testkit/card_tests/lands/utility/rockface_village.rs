//! `cards/lands/utility/rockface_village.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rockface Village prints `{{T}}: Add {{C}}.`, `{{T}}: Add {{R}}. Spend this mana only to cast a creature spell.`, and `{{R}}, {{T}}: Target Lizard, Mouse, Otter, or Raccoon you control gets +1/+0 and gains haste until end of turn. Activate only as a sorcery.`
///
/// Under `Coverage::Implemented`, activating ability 1 produces red mana restricted to creature spells, which is tracked in `pool.restricted()` rather than general available mana.
/// With only a Wolf on the battlefield, ability 2 is withheld from `legal.abilities` for lack of a legal target even when `{{R}}` is floating.
#[test]
fn rockface_village_produces_creature_restricted_red_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rockface_village(), mountain(), young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let village = on_battlefield(&engine, p0, rockface_village()).expect("village on battlefield");

    // Float {{R}} from Mountain while keeping Rockface Village untapped.
    tap_mana_except(&mut engine, p0, village);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(!is_tapped(&engine, village));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(village, 0)),
        "ability 0 (colorless mana) is offered"
    );
    assert!(
        legal.abilities.contains(&(village, 1)),
        "ability 1 (restricted creature mana) is offered"
    );
    assert!(
        !legal.abilities.contains(&(village, 2)),
        "ability 2 is not offered when the battlefield has no legal tribal target"
    );

    activate(&mut engine, p0, rockface_village(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "only the mountain's unrestricted red mana appears in the available pool"
    );
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Red);
    assert_eq!(pool.total(), 2);
    assert!(is_tapped(&engine, village));
}

/// Rockface Village: "{R}, {T}: Target Lizard, Mouse, Otter, or Raccoon you
/// control gets +1/+0 and gains haste until end of turn. Activate only as a
/// sorcery."
#[test]
fn rockface_village_pumps_a_lizard_and_gives_it_haste() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4205, mountain())
        .battlefield(
            0,
            &[
                rockface_village(),
                mountain(),
                viashino_grappler(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lizard = on_battlefield(&engine, p0, viashino_grappler()).expect("the Lizard");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves");
    let mountain_id = on_battlefield(&engine, p0, mountain()).expect("the Mountain");
    tap_mana_where(&mut engine, p0, |id| id == mountain_id);
    activate(&mut engine, p0, rockface_village(), 2);
    answer_target(&mut engine, lizard, &[elves]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, lizard), (4, 1), "3/1 and +1/+0");
    assert!(keywords(&engine, lizard).contains(KeywordSet::HASTE));
    assert_eq!(pt(&engine, elves), (1, 1));
}
