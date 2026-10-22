//! `cards/enchantments/mv_4/tombstone_stairwell.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tombstone_stairwell() -> CardIndex {
    card_index("9aaf1149-eccb-434a-a1a7-14b93fa50141")
}

/// Tombstone Stairwell (`{2}{B}{B}`, `Coverage::Partial`): World Enchantment
/// with cumulative upkeep `{1}{B}` that creates a 2/2 black Tombspawn Zombie
/// token with haste for each creature card in your graveyard at each upkeep and
/// destroys all Tombspawn tokens at each end step.
#[test]
fn tombstone_stairwell_creates_tombspawn_tokens_on_upkeep_and_destroys_them_at_end_step() {
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
        .start();
    keep_mulligans(&mut engine);
    let (seat, _) = walk_to_price(&mut engine);
    assert_eq!(seat, p0);
    pay_the_price_with(&mut engine, p0, |_| true);
    reach_main_phase(&mut engine, p0);

    // Put Grizzly Bears and Gray Ogre into p0's graveyard.
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    bury(&mut engine, &[bear, ogre]);

    // Advance to p1's upkeep: Tombstone Stairwell triggers at each upkeep and creates 2 Tombspawn tokens for p0!
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        2,
        "2 Tombspawn tokens for 2 creature cards in graveyard"
    );
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::HASTE));

    // At the end step of p1's turn, those Tombspawn tokens are destroyed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::End && stack_is_empty(e)
    });
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "all Tombspawn tokens destroyed at end step"
    );
}
