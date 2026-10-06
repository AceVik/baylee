//! `cards/lands/utility/hanweir_battlements.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hanweir Battlements prints `{{T}}: Add {{C}}.`, `{{R}}, {{T}}: Target creature gains haste until end of turn.`, and `{{3}}{{R}}{{R}}, {{T}}: If you both own and control this land and a creature named Hanweir Garrison, exile them, then meld them into Hanweir, the Writhing Township.`
///
/// Under `Coverage::Partial`, the mana ability and haste activation are implemented while the meld clause is omitted.
/// With `{{3}}{{R}}{{R}}` floating from five `mountain()` lands, a creature present (`llanowar_elves()`), and Hanweir Battlements untapped, ability 2 is not offered in `legal.abilities`.
/// Activating ability 1 grants haste to the target creature until end of turn and taps the land.
#[test]
fn hanweir_battlements_grants_haste_and_omits_meld_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                hanweir_battlements(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let battlements = on_battlefield(&engine, p0, hanweir_battlements())
        .expect("hanweir battlements on battlefield");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf on battlefield");

    assert!(
        !keywords_of(&engine, elf).contains(KeywordSet::HASTE),
        "creature does not have haste initially"
    );

    // Float {{5}} red mana from the five Mountains while keeping Hanweir Battlements untapped.
    tap_mana_except(&mut engine, p0, battlements);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        5
    );
    assert!(!is_tapped(&engine, battlements));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(battlements, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(battlements, 1)),
        "ability 1 ({{R}}, {{T}}: Target creature gains haste) is offered"
    );
    assert!(
        !legal.abilities.contains(&(battlements, 2)),
        "ability 2 (meld) is omitted under `Coverage::Partial`"
    );

    activate(&mut engine, p0, hanweir_battlements(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords_of(&engine, elf).contains(KeywordSet::HASTE),
        "creature gained haste"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        4
    );
    assert!(is_tapped(&engine, battlements));
}
