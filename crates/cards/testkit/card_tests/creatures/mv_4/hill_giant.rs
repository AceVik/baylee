//! `cards/creatures/mv_4/hill_giant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Hill Giant prints no rules text: a red {3}{R} Giant, 3/3, with no keyword
/// and no ability for anyone to press. The projection is the whole card.
#[test]
fn hill_giant_is_a_vanilla_red_three_three_giant() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("seated");

    let c = engine.state().object(giant).unwrap().characteristics();
    assert_eq!((c.power, c.toughness), (Some(3), Some(3)));
    assert_eq!(c.types, TypeSet::CREATURE, "a creature and nothing else");
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::GIANT)
            && c.subtypes.iter().count() == 1,
        "exactly the one subtype Giant"
    );
    assert_eq!(
        c.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Red]),
        "red, from its {{3}}{{R}}"
    );
    assert_eq!(c.mana_cost, baylee_core::mana!("{3}{R}"));
    assert_eq!(c.keywords, KeywordSet::default(), "no keyword");
    assert!(
        !priority_offer(&engine)
            .abilities
            .iter()
            .any(|&(id, _)| id == giant),
        "and no ability to activate"
    );
}

/// Played in combat, the 3/3 is only what it is printed as: unblocked it takes
/// three life, and blocked by a Savannah Lions (2/1) it kills the Lions and
/// survives with two damage marked.
#[test]
fn hill_giant_deals_three_unblocked_and_outlasts_a_savannah_lions() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hill_giant()])
        .battlefield(1, &[savannah_lions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("seated");
    let lions = on_battlefield(&engine, p1, savannah_lions()).expect("seated");
    let blocks = attack_and_collect_blocks(&mut engine, giant, p1);
    assert!(
        blocks.iter().any(|b| b.blocker == lions),
        "the Lions may block: {blocks:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(lions, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert!(
        in_graveyard(&engine, p1, savannah_lions()).is_some(),
        "3 damage to a 2/1"
    );
    assert_eq!(
        engine.state().object(giant).unwrap().damage,
        2,
        "the Lions dealt its 2 and the 3/3 is still there"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "blocked: no damage through"
    );

    let mut open = Duel::new(SEED, forest())
        .battlefield(0, &[hill_giant()])
        .start();
    keep_mulligans(&mut open);
    reach_main_phase(&mut open, p0);
    let giant = on_battlefield(&open, p0, hill_giant()).expect("seated");
    let blocks = attack_and_collect_blocks(&mut open, giant, p1);
    assert!(blocks.is_empty(), "nobody to block");
    pass_until(&mut open, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(open.state().players[1].life, 17, "three unblocked");
}
