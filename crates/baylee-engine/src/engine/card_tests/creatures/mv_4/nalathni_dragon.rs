//! `cards/creatures/mv_4/nalathni_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nalathni Dragon (0887dc95-3e75-4f72-bf05-4067d3785376) — "Flying; banding.
/// {R}: This creature gets +1/+0 until end of turn. If this ability has been
/// activated four or more times this turn, sacrifice this creature at the
/// beginning of the next end step."
#[test]
fn nalathni_dragon_pumps_and_is_sacrificed_after_four_activations() {
    let p0 = PlayerId::new(0);
    let dragon_card = card_index("0887dc95-3e75-4f72-bf05-4067d3785376");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[dragon_card, mountain(), mountain(), mountain(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let dragon = on_battlefield(&engine, p0, dragon_card).expect("Nalathni Dragon is out");
    assert_eq!(pt(&engine, dragon), (1, 1));
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, dragon).contains(KeywordSet::BANDING));

    tap_all_mana(&mut engine, p0);
    for _ in 0..4 {
        activate(&mut engine, p0, dragon_card, 0);
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dragon), (5, 1), "+1/+0 four times");

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    assert_eq!(
        engine.state().object(dragon).map(|o| o.zone),
        Some(Zone::Battlefield),
        "nothing happens before the trigger resolves"
    );
    assert!(!stack_is_empty(&engine), "the sacrifice is on the stack");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(dragon).map(|o| o.zone),
        Some(Zone::Graveyard),
        "sacrificed at the beginning of the end step"
    );
}
