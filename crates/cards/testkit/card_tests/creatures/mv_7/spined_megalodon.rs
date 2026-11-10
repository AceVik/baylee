//! `cards/creatures/mv_7/spined_megalodon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn spined_megalodon() -> CardIndex {
    card_index("3a124fe5-f525-43e5-9746-501c68a98330")
}

/// Spined Megalodon — a 5/7 with hexproof for {5}{U}{U}.
#[test]
fn spined_megalodon_costs_seven_and_is_a_five_seven() {
    let mut engine = table7(&lots(island(), 7), &[spined_megalodon()], &[]);
    let shark = cast_and_resolve(&mut engine, spined_megalodon());
    assert_body(&engine, shark, (5, 7), KeywordSet::HEXPROOF);
}

/// Hexproof: their Bolt may aim at the Ogre and not at the Megalodon.
#[test]
fn spined_megalodon_hexproof_keeps_their_bolt_off_it() {
    let (menu, shark, ogre) = bolt_menu(spined_megalodon(), 1);
    assert!(menu.contains(&ogre));
    assert!(!menu.contains(&shark));
}

/// "Whenever this creature attacks, scry 1." The top card is looked at, and
/// sent to the bottom it is no longer the top of the library.
#[test]
fn spined_megalodon_scries_one_when_it_attacks() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&[spined_megalodon()], &[], &[]);
    let shark = on_battlefield(&engine, p0, spined_megalodon()).expect("out");
    let library = library_size(&engine, p0);
    let top = engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .copied()
        .expect("a library");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shark, Defender::Player(PlayerId::new(1)))],
            },
        )
        .expect("the Megalodon attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped here")
    };
    assert_eq!(cards, vec![top], "the one top card is looked at");
    engine
        .apply(p0, look_answer(&cards, &cards))
        .expect("bottom it");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert_eq!(library_size(&engine, p0), library, "nothing drawn or lost");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).first(),
        Some(&top),
        "it went to the bottom of the library"
    );
}
