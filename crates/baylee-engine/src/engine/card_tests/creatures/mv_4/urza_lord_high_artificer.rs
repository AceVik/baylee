//! `cards/creatures/mv_4/urza_lord_high_artificer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Urza, Lord High Artificer — {2}{U}{U} 1/4 — the two printed sentences that
/// are implemented, both played in one first main phase. The entry trigger
/// makes a 0/0 Construct *artifact creature* whose own static grows it by one
/// for each artifact its controller controls, so the token standing beside the
/// Sol Ring has to be a 2/2: one would mean it never counted itself, three that
/// the opponent's Sol Ring across the table was counted too. And "Tap an
/// untapped artifact you control: Add {U}" is a mana ability whose cost names
/// no artifact of its own, so the question, the menu it publishes and the {U}
/// it pays are all read off the same activation. The `{5}` activation is the
/// `Coverage::Partial` gap and is never pressed.
#[allow(clippy::too_many_lines)] // the token's body read off the board, then its mana ability
#[test]
fn urza_builds_a_construct_that_counts_your_artifacts_and_taps_one_for_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), quiet_artifact()],
        )
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[urza_lord_high_artificer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Islands pay {2}{U}{U}. The Sol Ring is kept back through both
    // taps below: it is the "untapped artifact you control" Urza's own mana
    // ability charges, and it is what the Construct counts.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    cast_with_floating(&mut engine, p0, urza_lord_high_artificer());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && !tokens_of(e, p0).is_empty()
    });

    let urza = on_battlefield(&engine, p0, urza_lord_high_artificer()).expect("Urza resolved");
    assert_eq!(pt(&engine, urza), (1, 4), "the body the card prints");

    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        1,
        "one Construct — a token left at 0/0 would have died to CR 704.5f"
    );
    let construct = tokens[0];
    let kinds = types(&engine, construct);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "the token counts itself because it is an artifact creature: {kinds:?}"
    );
    assert_eq!(
        pt(&engine, construct),
        (2, 2),
        "+1/+1 for each artifact *you* control: the Sol Ring and the Construct \
         itself, and nothing for the Sol Ring the opponent controls"
    );

    // "Tap an untapped artifact you control: Add {U}". Tapping everything
    // else first settles the pool: a mana ability needs no mana, but the
    // offer is read once nothing else is floating.
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring stands");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring stands");
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(urza, 1)),
        "there is an untapped artifact to tap, so the printed mana ability is \
         offered: {:?}",
        legal.abilities
    );
    let before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Blue);

    activate(&mut engine, p0, urza_lord_high_artificer(), 1);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the tap is chosen before it is paid, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostTap,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, and the cost asks once");
    assert!(
        options.contains(&ring),
        "the untapped artifact you control is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "an artifact this seat does not control is not: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the artifact the question offered pays the cost");

    assert!(is_tapped(&engine, ring), "the artifact that was named paid");
    assert!(
        !is_tapped(&engine, construct),
        "and the Construct beside it never moved"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        before + 1,
        "{{U}} reached the pool the moment the answer landed"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        u64::from(before) + 1,
        "one mana, off one tap, and nothing else came with it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}
