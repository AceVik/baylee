//! `cards/lands/utility/mudflat_village.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mudflat Village prints `{{T}}: Add {{C}}.`, `{{T}}: Add {{B}}. Spend this mana only to cast a creature spell.`, and `{{1}}{{B}}, {{T}}, Sacrifice this land: Return target Bat, Lizard, Rat, or Squirrel card from your graveyard to your hand.`
///
/// Under `Coverage::Implemented`, activating ability 1 produces black mana restricted to casting creature spells.
/// Because this mana carries a spending restriction, it appears in `pool.restricted()` rather than `pool.available(ManaColor::Black)`.
/// With an empty graveyard, ability 2 is withheld from `legal.abilities` for lack of a legal target even when `{{1}}{{B}}` is floating.
#[test]
fn mudflat_village_produces_creature_restricted_black_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mudflat_village(), forest(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let village =
        on_battlefield(&engine, p0, mudflat_village()).expect("mudflat village on battlefield");

    // Float {{1}}{{B}} from basic lands while keeping Mudflat Village untapped.
    tap_mana_except(&mut engine, p0, village);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

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
        "ability 2 is not offered when the graveyard has no legal target"
    );

    activate(&mut engine, p0, mudflat_village(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "only the swamp's unrestricted black mana appears in the available pool"
    );
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Black);
    assert!(is_tapped(&engine, village));
}

/// Mudflat Village: "{1}{B}, {T}, Sacrifice this land: Return target Bat,
/// Lizard, Rat, or Squirrel card from your graveyard to your hand." Sewer
/// Rats (a Rat) is on offer; Llanowar Elves in the same graveyard is not.
#[test]
fn mudflat_village_returns_a_rat_and_not_an_elf_from_the_graveyard() {
    let p0 = PlayerId::new(0);
    let village = card_index("aeeab1df-0b8b-4bc4-a5f9-aac413449bec");
    let rats = card_index("bf526a0d-65cc-445f-b2b8-19a2cfdd836b");
    let mut engine = Duel::new(2301, forest())
        .battlefield(0, &[village, swamp(), forest(), rats, llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let v = on_battlefield(&engine, p0, village).expect("village");
    let r = on_battlefield(&engine, p0, rats).expect("rats");
    let e = on_battlefield(&engine, p0, llanowar_elves()).expect("elves");
    bury(&mut engine, &[r, e]);
    tap_mana_where(&mut engine, p0, |id| id != v && id != r && id != e);

    activate(&mut engine, p0, village, 2);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&r), "a Rat card is a target");
    assert!(!options.contains(&e), "an Elf card is not");
    unf_aim_and_pay(&mut engine, p0, Some(r), None);

    assert!(in_hand(&engine, p0, rats).is_some(), "the Rats came back");
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(in_graveyard(&engine, p0, village).is_some(), "sacrificed");
}
