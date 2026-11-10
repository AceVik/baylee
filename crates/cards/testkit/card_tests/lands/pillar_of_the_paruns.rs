//! `cards/lands/pillar_of_the_paruns.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pillar of the Paruns: "{T}: Add one mana of any color. Spend this mana only to cast a multicolored spell."
/// Activating the land produces mana marked with a multicolored spell restriction.
/// Alongside a Swamp, the restricted blue mana successfully casts Baleful Strix.
#[test]
fn pillar_of_the_paruns_adds_restricted_mana_for_multicolored_spells() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(118, forest())
        .battlefield(0, &[pillar_of_the_paruns(), swamp()])
        .hand(0, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, pillar_of_the_paruns(), 0);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);

    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    let strix = in_hand(&engine, p0, baleful_strix()).expect("Strix in hand");
    assert!(
        legal.castable.contains(&strix),
        "Baleful Strix is castable with restricted mana"
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: strix })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, baleful_strix()).is_some(),
        "Baleful Strix resolved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "restricted mana was spent"
    );
}
