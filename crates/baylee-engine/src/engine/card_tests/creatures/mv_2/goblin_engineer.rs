//! `cards/creatures/mv_2/goblin_engineer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Engineer's activated half — "{R}, {T}, Sacrifice an artifact:
/// Return target artifact card with mana value 3 or less from your graveyard
/// to the battlefield" — since the search-to-graveyard ETB is the
/// `Coverage::Partial` gap. Three readings are struck by one activation: the
/// card that comes back is a Chromatic Lantern, mana value 3 exactly, so a
/// filter that read "less than three" would leave the ability with no target
/// at all; the identical Lantern in the opponent's graveyard is the "your"
/// the sentence prints; and the menu the sacrifice asks with holds the Sol
/// Ring this seat controls rather than the one across the table. Tapping to
/// float the {R} first is not scenery either — the ability is only offered
/// once the cost is payable, and it ends tapped with the red gone.
#[allow(clippy::too_many_lines)] // a sacrifice, a graveyard search and the permanent that arrives tapped
#[test]
fn goblin_engineer_trades_an_artifact_for_the_lantern_in_his_own_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // The backing deck is the card's own graveyard clause: nothing else in the
    // harness puts a named *artifact card* into a graveyard — an opening
    // battlefield holds permanents and `seed_graveyard` moves cards off the top
    // of a library — so the library has to be the printing the ability is
    // about. Chromatic Lantern is mana value 3 exactly, the number "3 or less"
    // has to include.
    let mut engine = Duel::new(SEED, chromatic_lantern())
        .battlefield(0, &[goblin_engineer(), mountain(), quiet_artifact()])
        // The same card and the same artifact across the table, so each
        // refusal is made by the sentence rather than by the board.
        .battlefield(1, &[forest(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);

    let engineer = on_battlefield(&engine, p0, goblin_engineer()).expect("the Engineer is out");
    let fodder = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let lantern =
        in_graveyard(&engine, p0, chromatic_lantern()).expect("my Lantern is in my graveyard");
    let their_lantern =
        in_graveyard(&engine, p1, chromatic_lantern()).expect("theirs is in their graveyard");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the Mountain is floating the {{R}} the cost asks for"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(engineer, 0)),
        "a card in the graveyard is a target and a red source is floating, so \
         the one line the Engineer prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, goblin_engineer(), 0);
    // CR 601.2c comes before CR 601.2h: the graveyard is asked about first and
    // the sacrifice second.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the ability targets, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&lantern),
        "mana value 3 is within \"3 or less\": {options:?}"
    );
    assert!(
        !options.contains(&their_lantern),
        "\"from your graveyard\" — the identical card across the table is not \
         offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lantern],
            },
        )
        .expect("the card the question offered is the card it takes");

    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is chosen before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "the artifact this seat controls is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&their_rock),
        "CR 701.21a: an opponent's artifact is not yours to sacrifice: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and it is the whole menu — the Engineer is a creature and the Mountain \
         a land"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the artifact the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, chromatic_lantern()).is_some(),
        "the ability returns the card from the graveyard to the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, chromatic_lantern()).is_none(),
        "and it is no longer a card in a graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "the sacrificed artifact is what the ability spent"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, chromatic_lantern()).is_some(),
        "the other seat's graveyard was never read, let alone moved from"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and neither was their artifact"
    );
    assert!(is_tapped(&engine, engineer), "{{T}} was part of the cost");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "and the {{R}} was spent, not merely floated"
    );
}
