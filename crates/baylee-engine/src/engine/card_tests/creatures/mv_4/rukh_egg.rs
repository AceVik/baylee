//! `cards/creatures/mv_4/rukh_egg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rukh Egg — `{3}{R}` 0/3: "When this creature dies, create a 4/4 red Bird
/// creature token with flying at the beginning of the next end step."
///
/// "Dies" is CR 700.4 and the Bird waits behind a delayed triggered ability
/// (CR 603.7): the Egg is destroyed in the first main phase and nothing has
/// arrived yet, and the end step brings one 4/4 flier.
#[test]
fn rukh_egg_makes_a_bird_at_the_next_end_step_after_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rukh_egg()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let egg = on_battlefield(&engine, p0, rukh_egg()).expect("seated");

    kill(&mut engine, egg);
    assert!(
        in_graveyard(&engine, p0, rukh_egg()).is_some(),
        "the Egg died (CR 700.4)"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "not yet: the Bird waits for the end step"
    );
    assert_eq!(
        engine.state().delayed.len(),
        1,
        "one delayed trigger (CR 603.7)"
    );

    pass_until(&mut engine, |e| !tokens_of(e, p0).is_empty());
    assert!(
        matches!(engine.state().turn.step, Step::End),
        "the Bird arrives at the end step"
    );
    let birds = tokens_of(&engine, p0);
    assert_eq!(birds.len(), 1, "one Bird");
    assert_eq!(pt(&engine, birds[0]), (4, 4), "a 4/4");
    assert!(
        keywords(&engine, birds[0]).contains(KeywordSet::FLYING),
        "with flying"
    );
    assert!(
        engine.state().delayed.is_empty(),
        "and the delayed trigger is spent"
    );
}

/// The card's trigger is "dies" and nothing else (CR 700.4), which is the
/// 2005-10-01 ruling: "If the Egg is exiled instead of being put into the
/// graveyard, no Bird is put onto the battlefield." Swords to Plowshares is
/// the exile, and no end step ever brings a Bird.
#[test]
fn rukh_egg_makes_no_bird_when_it_is_exiled_instead_of_dying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rukh_egg()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let egg = on_battlefield(&engine, p0, rukh_egg()).expect("seated");

    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    aim_at(&mut engine, p1, egg);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(egg).map(|o| o.zone),
        Some(Zone::Exile),
        "exiled rather than put into a graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, rukh_egg()).is_none(),
        "so it never died (CR 700.4)"
    );
    assert!(
        engine.state().delayed.is_empty(),
        "and no delayed trigger was made"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(
        tokens_of(&engine, p0).is_empty() && tokens_of(&engine, p1).is_empty(),
        "no Bird, for anybody, ever"
    );
}

/// The next 2005-10-01 ruling: "If the Egg is destroyed while under the
/// control of another player, the controller of the Egg gets the Bird." p1
/// takes p0's seated Egg with Control Magic, p0 destroys it with Lightning
/// Bolt, and the delayed trigger belongs to p1 when it dies, so p1's end
/// step makes the Bird.
#[test]
fn rukh_egg_gives_the_bird_to_whoever_controlled_it_when_it_died() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), rukh_egg()])
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[control_magic()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let egg = on_battlefield(&engine, p0, rukh_egg()).expect("p0's Egg");
    cast_from_hand(&mut engine, p1, control_magic());
    aim_at(&mut engine, p1, egg);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(egg).map(|o| o.controller),
        Some(p1),
        "p1 controls the Egg now"
    );

    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, egg);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, rukh_egg()).is_some(),
        "3 damage kills the 0/3, and it goes to its owner's graveyard"
    );

    pass_until(&mut engine, |e| !tokens_of(e, p1).is_empty());
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the Bird is not the owner's"
    );
    let birds = tokens_of(&engine, p1);
    assert_eq!(birds.len(), 1, "the controller at death gets it");
    assert_eq!(pt(&engine, birds[0]), (4, 4), "a 4/4");
    assert!(
        keywords(&engine, birds[0]).contains(KeywordSet::FLYING),
        "with flying"
    );
}
