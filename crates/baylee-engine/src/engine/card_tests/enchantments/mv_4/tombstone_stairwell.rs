//! `cards/enchantments/mv_4/tombstone_stairwell.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tombstone_stairwell() -> CardIndex {
    card_index("9aaf1149-eccb-434a-a1a7-14b93fa50141")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// A duel at the first upkeep, the cumulative upkeep paid, with Tombstone
/// Stairwell in play. P0's Grizzly Bears and Gray Ogre and P1's Hill Giant are
/// in their graveyards by the first main phase.
fn stairwell_with_graveyards() -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                tombstone_stairwell(),
                swamp(),
                swamp(),
                grizzly_bears(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    let (seat, _) = walk_to_price(&mut engine);
    assert_eq!(seat, p0);
    pay_the_price_with(&mut engine, p0, |_| true);
    reach_main_phase(&mut engine, p0);
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    bury(&mut engine, &[bear, ogre, giant]);
    engine
}

/// Walks into P1's upkeep, where the Stairwell triggers, and lets it resolve.
fn through_the_opponents_upkeep(engine: &mut Engine<RegistryLookup>) {
    let p1 = PlayerId::new(1);
    pass_until(engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(engine, stack_is_empty);
}

/// Tombstone Stairwell (`{2}{B}{B}`, `Coverage::Partial`): at each upkeep each
/// player creates a 2/2 Tombspawn with haste for each creature card in their
/// own graveyard, and the tokens are destroyed at the end step. Both players
/// are served, each from their own graveyard.
#[test]
fn tombstone_stairwell_serves_both_players_and_destroys_the_tokens_at_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = stairwell_with_graveyards();

    through_the_opponents_upkeep(&mut engine);
    let mine = tokens_of(&engine, p0);
    let theirs = tokens_of(&engine, p1);
    assert_eq!(
        mine.len(),
        2,
        "2 Tombspawn tokens for 2 creature cards in P0's graveyard"
    );
    assert_eq!(
        theirs.len(),
        1,
        "1 Tombspawn token for the 1 creature card in P1's graveyard"
    );
    assert_eq!(pt(&engine, mine[0]), (2, 2));
    assert!(keywords(&engine, mine[0]).contains(KeywordSet::HASTE));
    assert_eq!(pt(&engine, theirs[0]), (2, 2));

    // At the end step of that turn the tokens of both players are destroyed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::End && stack_is_empty(e)
    });
    assert!(
        tokens_of(&engine, p0).is_empty() && tokens_of(&engine, p1).is_empty(),
        "all Tombspawn tokens destroyed at the end step"
    );
}

/// Tombstone Stairwell — "when this enchantment leaves the battlefield,
/// destroy all tokens": well before the end step, if it goes first.
#[test]
fn tombstone_stairwell_destroys_its_tokens_when_it_leaves_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = stairwell_with_graveyards();

    through_the_opponents_upkeep(&mut engine);
    assert_eq!(tokens_of(&engine, p0).len(), 2);

    let stairwell = on_battlefield(&engine, p0, tombstone_stairwell()).unwrap();
    kill(&mut engine, stairwell);
    assert!(
        tokens_of(&engine, p0).is_empty() && tokens_of(&engine, PlayerId::new(1)).is_empty(),
        "the tokens go with the enchantment"
    );
    assert_ne!(
        engine.state().turn.step,
        Step::End,
        "it was the leaves-the-battlefield trigger and not the end step"
    );
}

/// Tombstone Stairwell — cumulative upkeep: unpaid, it is sacrificed.
#[test]
fn tombstone_stairwell_is_sacrificed_when_its_cumulative_upkeep_goes_unpaid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tombstone_stairwell(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    let (seat, _) = walk_to_price(&mut engine);
    assert_eq!(seat, p0);
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("the price can be refused");
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, tombstone_stairwell()).is_none());
    assert!(in_graveyard(&engine, p0, tombstone_stairwell()).is_some());
}
