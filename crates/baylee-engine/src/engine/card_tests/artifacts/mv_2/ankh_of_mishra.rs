//! `cards/artifacts/mv_2/ankh_of_mishra.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ankh of Mishra: "Whenever a land enters, this artifact deals 2 damage to
/// that land's controller." The controller of the *land*, not of the Ankh:
/// p0 owns the Ankh and still takes 2 for their own land drop, and p1 takes
/// 2 for theirs on the following turn — p0's total does not move again.
#[test]
fn ankh_of_mishra_deals_2_to_whichever_player_a_land_enters_for() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ankh_of_mishra()])
        .hand(0, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before0 = life_of(&engine, p0);
    play_land(&mut engine, p0, island());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p0),
        before0 - 2,
        "the Ankh's own controller takes 2 for their own land"
    );

    reach_their_main_phase(&mut engine, p1);
    let before1 = life_of(&engine, p1);
    play_land(&mut engine, p1, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p1),
        before1 - 2,
        "and the opponent takes 2 for theirs — the land's controller, not \
         the Ankh's"
    );
    assert_eq!(
        life_of(&engine, p0),
        before0 - 2,
        "p0's life did not move a second time"
    );
}

/// Circle of Protection: Artifacts: "{2}: The next time an artifact source
/// of your choice would deal damage to you this turn, prevent that damage."
///
/// Two halves, one activation. The printed price is read first: one floating
/// white is not {2} and the ability is not offered; two Plains pay it. The
/// source choice (CR 609.7a) offers the two artifacts across the table and
/// not the Elf — "an artifact source" is a filter. And the shield then takes
/// the whole 2 of the Ankh's landfall trigger, which is the "next time" and
/// the "to you" of the sentence (CR 615.1).
#[test]
fn circle_of_protection_artifacts_offers_only_artifacts_and_prevents_their_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[circle_of_protection_artifacts(), plains(), plains()])
        .hand(0, &[forest()])
        .battlefield(1, &[ankh_of_mishra(), quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let circle =
        on_battlefield(&engine, p0, circle_of_protection_artifacts()).expect("the Circle is out");
    let plains = all_on_battlefield(&engine, p0, plains());
    let ankh = on_battlefield(&engine, p1, ankh_of_mishra()).expect("their Ankh");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Ring");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");
    let before = life_of(&engine, p0);

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: plains[0] })
        .unwrap();
    assert!(
        !priority_offer(&engine).abilities.contains(&(circle, 0)),
        "one mana is not the printed {{2}}"
    );
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: plains[1] })
        .unwrap();
    assert!(
        priority_offer(&engine).abilities.contains(&(circle, 0)),
        "two Plains pay it"
    );
    activate(&mut engine, p0, circle_of_protection_artifacts(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        options,
        choice,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the source choice")
    };
    assert_eq!(player, p0, "the ability's controller chooses the source");
    let offered: Vec<ObjectId> = options.iter().map(|candidate| candidate.object).collect();
    assert!(
        offered.contains(&ankh),
        "the Ankh is an artifact source: {offered:?}"
    );
    assert!(
        offered.contains(&rock),
        "and so is the Sol Ring: {offered:?}"
    );
    assert!(
        !offered.contains(&elf),
        "a creature is not an artifact source: {offered:?}"
    );
    let chosen = options
        .iter()
        .copied()
        .find(|candidate| candidate.object == ankh)
        .expect("the Ankh is offered");
    engine
        .apply(
            p0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: chosen,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().shields.len(),
        1,
        "one shield, waiting for the Ankh"
    );

    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p0),
        before,
        "the Ankh's 2 to p0, all of it prevented"
    );
    assert!(
        engine.state().shields.is_empty(),
        "and the \"next time\" is spent"
    );
}
