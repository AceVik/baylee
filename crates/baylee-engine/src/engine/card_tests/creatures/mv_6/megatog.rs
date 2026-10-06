//! `cards/creatures/mv_6/megatog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Megatog — {4}{R}{R} 3/4 Atog: "Sacrifice an artifact: This creature gets
/// +3/+3 and gains trample until end of turn."
///
/// The filter is the whole card, so the sacrifice menu is read before it is
/// answered: it holds the one artifact this seat controls and nothing else on
/// the table — not the Elf beside it, not the Megatog itself, and not the Sol
/// Ring across the table, which is the same card under a controller that is
/// not the one paying (CR 701.21a). `(6, 7)` with trample is then the only
/// body that reads both halves of the printed effect, and a turn walked
/// afterwards is what tells "until end of turn" from a permanent grant.
#[test]
#[allow(clippy::too_many_lines)]
fn megatog_eats_an_artifact_of_its_own_side_for_three_and_trample() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                quiet_artifact(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[megatog()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {4}{R}{R} off the six Mountains. The Sol Ring is named as the printing
    // kept back: its whole price is its own {T} (#159), so `tap_all_mana`
    // would have spent the very artifact this test is about to sacrifice.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        6,
        "six Mountains tapped, six red — and the Sol Ring still standing"
    );
    cast_with_floating(&mut engine, p0, megatog());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let tog = on_battlefield(&engine, p0, megatog()).expect("the Megatog resolved");
    let fodder = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(pt(&engine, tog), (3, 4), "the body the card prints");
    assert!(
        !keywords(&engine, tog).contains(KeywordSet::TRAMPLE),
        "nothing has been eaten yet, so the grant has not happened"
    );

    activate(&mut engine, p0, megatog(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which artifact, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert_eq!(
        options,
        vec![fodder],
        "the one artifact this seat controls is the whole menu: {options:?}"
    );
    assert!(
        !options.contains(&tog),
        "the Megatog is an Atog and no artifact, so it cannot eat itself: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the Elf is a permanent of mine and still no artifact: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "CR 701.21a: an opponent's artifact is not yours to sacrifice: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the artifact the question offered pays the cost");

    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "a sacrificed artifact goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "pumping a creature is no mana ability, so the ability is on the stack"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        pt(&engine, tog),
        (6, 7),
        "+3/+3 on the printed 3/4 — one sacrifice, one pump"
    );
    assert!(
        keywords(&engine, tog).contains(KeywordSet::TRAMPLE),
        "and the printed trample, from the same granted set"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the pump belongs to the Megatog and not to the board"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the artifact the ability did not name never moved"
    );

    // "until end of turn": the Megatog is still standing a turn later and the
    // pump is not — a static would still read (6, 7) here.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, megatog()).is_some(),
        "the creature outlived the turn, so what left was the grant"
    );
    assert_eq!(
        pt(&engine, tog),
        (3, 4),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        !keywords(&engine, tog).contains(KeywordSet::TRAMPLE),
        "and the keyword left with it"
    );
}
