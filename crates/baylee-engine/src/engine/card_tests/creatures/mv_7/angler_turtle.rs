//! `cards/creatures/mv_7/angler_turtle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn angler_turtle() -> CardIndex {
    card_index("a09da13e-f8a3-410e-941a-9912d2cfe7de")
}

/// Angler Turtle — a 5/7 hexproof for {5}{U}{U}.
#[test]
fn angler_turtle_costs_seven_and_is_a_hexproof_five_seven() {
    let mut engine = table7(&lots(island(), 7), &[angler_turtle()], &[]);
    let turtle = cast_and_resolve(&mut engine, angler_turtle());
    assert_body(&engine, turtle, (5, 7), KeywordSet::HEXPROOF);
}

/// Hexproof: the opponent's Lightning Bolt may aim at the Ogre beside the
/// Turtle and not at the Turtle.
#[test]
fn angler_turtle_hexproof_keeps_their_bolt_off_it() {
    let (menu, turtle, ogre) = bolt_menu(angler_turtle(), 1);
    assert!(menu.contains(&ogre), "the Ogre is on the menu");
    assert!(!menu.contains(&turtle), "the hexproof Turtle is not");
}

/// "Creatures your opponents control attack each combat if able": the
/// opponent's Ogre is required on their turn and an empty declaration is
/// refused; on the Turtle controller's own turn nothing is required of its
/// side.
#[test]
fn angler_turtle_forces_the_opponents_creatures_to_attack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[angler_turtle()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("the Ogre is out");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { required, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped here")
    };
    assert!(required.is_empty(), "the Turtle's own side is not forced");
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .expect("no attack is fine for the Turtle's controller");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let Pending::ChooseAttackers { required, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped here")
    };
    assert!(required.contains(&ogre), "the Ogre must attack if able");
    assert!(
        engine
            .apply(p1, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "an empty declaration disobeys the requirement"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(p0))],
            },
        )
        .expect("attacking is the legal declaration");
}
