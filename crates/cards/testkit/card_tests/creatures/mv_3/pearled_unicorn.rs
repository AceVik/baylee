//! `cards/creatures/mv_3/pearled_unicorn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pearled Unicorn prints no rules text: a white {2}{W} Unicorn, 2/2, with no
/// keyword and no ability for anyone to press. The projection is the whole card.
#[test]
fn pearled_unicorn_is_a_vanilla_white_two_two_unicorn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[pearled_unicorn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let unicorn = on_battlefield(&engine, p0, pearled_unicorn()).expect("seated");

    let c = engine.state().object(unicorn).unwrap().characteristics();
    assert_eq!((c.power, c.toughness), (Some(2), Some(2)));
    assert_eq!(c.types, TypeSet::CREATURE, "a creature and nothing else");
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::UNICORN)
            && c.subtypes.iter().count() == 1,
        "exactly the one subtype Unicorn"
    );
    assert_eq!(
        c.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::White]),
        "white, from its {{2}}{{W}}"
    );
    assert_eq!(c.mana_cost, baylee_core::mana!("{2}{W}"));
    assert_eq!(c.keywords, KeywordSet::default(), "no keyword");
    assert!(
        !priority_offer(&engine)
            .abilities
            .iter()
            .any(|&(id, _)| id == unicorn),
        "and no ability to activate"
    );
}

/// A 2/2 is a 2/2 in combat: blocked by a Savannah Lions (2/1), each deals its
/// two and each dies, with no first strike, no damage prevented and no
/// toughness to spare.
#[test]
fn pearled_unicorn_and_a_savannah_lions_kill_each_other() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[pearled_unicorn()])
        .battlefield(1, &[savannah_lions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let unicorn = on_battlefield(&engine, p0, pearled_unicorn()).expect("seated");
    let lions = on_battlefield(&engine, p1, savannah_lions()).expect("seated");
    let blocks = attack_and_collect_blocks(&mut engine, unicorn, p1);
    assert!(
        blocks.iter().any(|b| b.blocker == lions),
        "the Lions may block: {blocks:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(lions, unicorn)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert!(
        in_graveyard(&engine, p1, savannah_lions()).is_some(),
        "2 damage to a 2/1"
    );
    assert!(
        in_graveyard(&engine, p0, pearled_unicorn()).is_some(),
        "and 2 damage back to a 2/2"
    );
}
