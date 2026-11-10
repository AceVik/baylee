//! `cards/creatures/mv_1/savannah_lions.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Savannah Lions prints no rules text: a white {W} Cat, 2/1, with no keyword
/// and no ability for anyone to press. The projection is the whole card.
#[test]
fn savannah_lions_is_a_vanilla_white_two_one_cat() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[savannah_lions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("seated");

    let c = engine.state().object(lions).unwrap().characteristics();
    assert_eq!((c.power, c.toughness), (Some(2), Some(1)));
    assert_eq!(c.types, TypeSet::CREATURE, "a creature and nothing else");
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::CAT)
            && c.subtypes.iter().count() == 1,
        "exactly the one subtype Cat"
    );
    assert_eq!(
        c.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::White]),
        "white, from its {{W}}"
    );
    assert_eq!(c.mana_cost, baylee_core::mana!("{W}"));
    assert_eq!(c.keywords, KeywordSet::default(), "no keyword");
    assert!(
        !priority_offer(&engine)
            .abilities
            .iter()
            .any(|&(id, _)| id == lions),
        "and no ability to activate"
    );
}

/// Unblocked, the 2/1 takes two life; blocked by a Hill Giant it deals its two
/// and dies, the Giant (3/3) walking away with two damage marked.
#[test]
fn savannah_lions_deals_two_unblocked_and_dies_to_a_hill_giant() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let hill_giant = card_index("342199e0-15b6-4824-83da-25caef2592b3");

    let mut open = Duel::new(SEED, forest())
        .battlefield(0, &[savannah_lions()])
        .start();
    keep_mulligans(&mut open);
    reach_main_phase(&mut open, p0);
    let lions = on_battlefield(&open, p0, savannah_lions()).expect("seated");
    let blocks = attack_and_collect_blocks(&mut open, lions, p1);
    assert!(blocks.is_empty(), "nobody to block");
    pass_until(&mut open, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(open.state().players[1].life, 18, "two unblocked");

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[savannah_lions()])
        .battlefield(1, &[hill_giant])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("seated");
    let giant = on_battlefield(&engine, p1, hill_giant).expect("seated");
    let blocks = attack_and_collect_blocks(&mut engine, lions, p1);
    assert!(blocks.iter().any(|b| b.blocker == giant), "{blocks:?}");
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, lions)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert!(in_graveyard(&engine, p0, savannah_lions()).is_some());
    assert_eq!(engine.state().object(giant).unwrap().damage, 2);
    assert_eq!(engine.state().players[1].life, 20, "blocked: none through");
}
