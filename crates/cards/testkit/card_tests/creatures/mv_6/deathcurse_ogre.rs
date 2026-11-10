//! `cards/creatures/mv_6/deathcurse_ogre.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Deathcurse Ogre prints one line: "When this creature dies, each player loses
/// 3 life." The word that carries the card is "each player", so the scenario
/// reads *both* seats after the Ogre has actually died — a trigger that had
/// been written as "target opponent" would satisfy every count taken on p1's
/// side of the table while leaving its own controller untouched. Six Swamps pay
/// the printed `{5}{B}`, which is what makes this a body that arrived by being
/// cast rather than a permanent seated on the battlefield, and the life totals
/// are read once before the death so that the three lost points are the
/// trigger's and not the format's.
#[test]
fn deathcurse_ogre_makes_every_player_lose_three_life_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 6])
        .hand(0, &[deathcurse_ogre()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, deathcurse_ogre());
    pass_until(&mut engine, stack_is_empty);
    let ogre = on_battlefield(&engine, p0, deathcurse_ogre()).expect("the Ogre resolved");
    assert_eq!(pt(&engine, ogre), (3, 3), "the body the card prints");
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (20, 20),
        "the Ogre entering costs nobody any life; the trigger is a death, not an entry"
    );

    // The harness door rather than a removal spell: what this test reads is the
    // *dying*, and every card that dies here dies the same way whatever killed
    // it (CR 704.5g).
    kill(&mut engine, ogre);

    assert!(
        in_graveyard(&engine, p0, deathcurse_ogre()).is_some(),
        "the Ogre died, which is the whole condition the trigger waits on"
    );
    assert_eq!(
        engine.state().players[0].life,
        17,
        "\"each player\" includes the Ogre's own controller, who loses the \
         three points as much as anybody"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "and the opponent across the table: a sentence read as \"target \
         opponent\" would have left the controller at 20"
    );
}
