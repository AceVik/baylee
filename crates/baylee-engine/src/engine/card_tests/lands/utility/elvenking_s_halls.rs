//! `cards/lands/utility/elvenking_s_halls.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Elvenking's Halls prints `This land enters tapped`, `{T}: Add {G} or {U}`, and `{2}{G}{U}, {T},
/// Sacrifice this land: Put two +1/+1 counters on target Elf you control. Activate only as a sorcery.`
/// The card is marked `Coverage::Implemented`.
/// After entering tapped and untapping on the next turn, floating `{2}{G}{U}` allows activating ability
/// index 1 to sacrifice the land and place two `CounterKind::P1P1` counters onto controlled `llanowar_elves`.
#[test]
fn elvenkings_halls_sacrifices_to_put_two_counters_on_controlled_elf() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), island(), island(), llanowar_elves()],
        )
        .hand(0, &[elvenking_s_halls()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, elvenking_s_halls());
    assert!(entered_tapped(&engine, land));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf exists");
    assert_eq!(pt(&engine, elf), (1, 1));

    tap_all_mana_but(&mut engine, p0, Some(elvenking_s_halls()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "two Forests, two Islands and the Elf, which taps for mana as well"
    );

    activate(&mut engine, p0, elvenking_s_halls(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice");
    };
    assert!(options.contains(&elf));
    assert!(on_battlefield(&engine, p0, elvenking_s_halls()).is_some());

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();

    assert!(in_graveyard(&engine, p0, elvenking_s_halls()).is_some());

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, elf, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, elf), (3, 3));
}
