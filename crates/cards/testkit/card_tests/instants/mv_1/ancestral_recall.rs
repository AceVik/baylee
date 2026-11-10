//! `cards/instants/mv_1/ancestral_recall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Deflecting Swat on an Ancestral Recall its caster aimed at themself
/// (#247): a "target player" spell keeps its player in `chosen_player`, and
/// the new one is written there. p0 takes the three cards.
#[test]
fn deflecting_swat_turns_an_ancestral_recall_onto_its_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[deflecting_swat()])
        .battlefield(1, &[island()])
        .hand(1, &[ancestral_recall()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, ancestral_recall());
    engine
        .apply(p1, PlayerAction::ChoosePlayer(p1))
        .expect("\"target player\": its own caster");
    let recall = on_stack(&engine, ancestral_recall()).expect("it is on the stack");
    let hands = |e: &Engine<RegistryLookup>| {
        [p0, p1].map(|p| e.state().zones.list(ZoneLocation::Hand(p)).len())
    };

    let Some(Pending::ChooseTargets {
        options,
        player_options,
        ..
    }) = swatted(&mut engine, p0, recall)
    else {
        panic!("the other player is a legal target, so the Swat asks")
    };
    assert_eq!(
        (options, player_options),
        (vec![], vec![p0]),
        "\"target player\": the other one, and no object"
    );
    let before = hands(&engine);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("the player offered");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hands(&engine),
        [before[0] + 3, before[1]],
        "p0 draws the three, and p1 none"
    );
}

/// Ancestral Recall — {U} — Instant: "Target player draws three cards."
///
/// The word that decides the card is "target": the three cards belong to the
/// player the spell names, so the scenario aims it across the table and reads
/// both libraries afterwards. The caster's own library is the control that
/// keeps "a draw" from passing as "a draw for everybody", and the target
/// question is read while it still stands, which is where "target player"
/// has to offer both seats rather than only the one across the table.
#[test]
fn ancestral_recall_draws_three_for_the_player_it_names_and_not_for_the_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[ancestral_recall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine_before = library_size(&engine, p0);
    let theirs_before = library_size(&engine, p1);
    let my_hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let their_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    cast_from_hand(&mut engine, p0, ancestral_recall());

    // The announcement is **atomic in this engine**, and that is worth
    // pinning rather than working around. CR 601.2a moves the card to the
    // stack before CR 601.2c asks for a target; `cast_wizard` asks first and
    // moves at the end, so while the question stands the card is still in
    // hand. Nothing can see the difference: no player is given priority
    // until CR 601.2i, and CR 115.5 makes a spell an illegal target of
    // itself, so the one list that would have held the card holds nothing
    // either way. If this assertion ever fails, the wizard has started
    // moving the card first and the reason above is the thing to re-read.
    assert!(
        on_stack(&engine, ancestral_recall()).is_none()
            && in_hand(&engine, p0, ancestral_recall()).is_some(),
        "the card is where it was until the wizard finishes"
    );

    // A target that is only ever a player asks `ChoosePlayer` rather than
    // `ChooseTargets`: two pendings, and the DSL spec picks between them —
    // `TargetSpec::AnyPlayer` here, against `AnyTarget`, which offers both
    // lists at once because CR 115.4 lets it name a creature too.
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!(
            "\"target player\" is a player choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat names the player");
    assert!(
        options.contains(&p0) && options.contains(&p1),
        "\"target player\" reaches either seat, its own included: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the opponent was one of the players the question offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        theirs_before - 3,
        "\"target player draws three cards\": three off the top of that player's library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand_before + 3,
        "and the cards are in the target's hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        my_hand_before - 1,
        "the caster's hand only lost the spell it cast"
    );
    assert_eq!(
        library_size(&engine, p0),
        mine_before,
        "and the caster's library never moved, which is what tells \"target \
         player\" from \"you\""
    );
    assert!(
        in_graveyard(&engine, p0, ancestral_recall()).is_some(),
        "the instant resolved and went to its owner's graveyard"
    );
    assert!(
        engine.state().players[0].mana_pool.total() == 0,
        "the {{U}} was paid"
    );
}
