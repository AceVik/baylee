//! `cards/artifacts/mv_0/fountain_of_youth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fountain of Youth prints exactly one line — "{2}, {T}: You gain 1 life" —
/// and the board is two Plains and the artifact, so both halves of the price
/// are readable straight off the state and nothing else on the table can move
/// a life total. The offer is read *after* the mana is already floating, which
/// is the only reading that tells a real {2} from an ability the engine
/// withheld for want of mana (CR 601.2h); the tapped artifact and the emptied
/// pool afterwards then say the price was actually paid rather than merely
/// printed.
#[test]
fn fountain_of_youth_taps_and_two_mana_for_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), fountain_of_youth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let fountain = on_battlefield(&engine, p0, fountain_of_youth()).expect("the Fountain is out");
    let life = engine.state().players[0].life;
    assert!(!is_tapped(&engine, fountain), "it starts untapped");

    // Two Plains pay the {2}. The Fountain prints no mana of its own, so its
    // {T} is still standing for the ability itself.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Plains, and the artifact taps for nothing"
    );
    assert!(!is_tapped(&engine, fountain), "nothing has tapped it yet");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(fountain, 0)),
        "with {{2}} floating, the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, fountain_of_youth(), 0);
    assert!(
        is_tapped(&engine, fountain),
        "{{T}} is half the cost and is paid as the ability is activated"
    );
    assert!(!stack_is_empty(&engine), "gaining life is no mana ability");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} that was floating is spent"
    );
    assert_eq!(
        engine.state().players[0].life,
        life + 1,
        "\"You gain 1 life\" — one activation, one life"
    );
    assert!(
        on_battlefield(&engine, p0, fountain_of_youth()).is_some(),
        "an activated ability costs the artifact nothing but its tap"
    );
}
