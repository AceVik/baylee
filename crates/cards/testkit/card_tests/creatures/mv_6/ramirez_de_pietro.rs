//! `cards/creatures/mv_6/ramirez_de_pietro.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ramirez `DePietro` is a `{3}{U}{B}{B}` legendary 4/3 Human Pirate whose whole
/// printed text is first strike (CR 702.7), and a keyword only exists in
/// combat. The blocking creature is chosen so the two damage steps tell each
/// other apart: Baleful Strix is a 1/1 with **deathtouch**, so if the damage
/// were dealt simultaneously its single point would be lethal and the Pirate
/// would die beside it. That the Pirate kills the Strix and stands untouched
/// afterwards is the whole of what first strike means — and the pair of
/// assertions cannot both hold on a board where the keyword was skipped.
#[test]
fn ramirez_de_pietro_kills_a_deathtouch_blocker_in_the_first_strike_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ramirez_de_pietro(), forest()])
        .battlefield(1, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    let pirate =
        on_battlefield(&engine, p0, ramirez_de_pietro()).expect("the Pirate is on the battlefield");
    let strix =
        on_battlefield(&engine, p1, baleful_strix()).expect("the Strix is on the battlefield");
    assert_eq!(pt(&engine, pirate), (4, 3), "the body the card prints");
    assert!(
        keywords(&engine, pirate).contains(KeywordSet::FIRST_STRIKE),
        "and the printed first strike reaches the permanent"
    );
    assert_eq!(
        pt(&engine, strix),
        (1, 1),
        "a printed 1/1 blocks it, so the only thing that could kill the Pirate \
         is the deathtouch damage the Strix must never get to deal"
    );

    // The Pirate attacks and the Strix blocks. `attack_and_collect_blocks`
    // declares the attacker and hands back the pairings the defending seat is
    // offered, which is the only place a legal block exists to be read.
    let blocks = attack_and_collect_blocks(&mut engine, pirate, p1);
    assert!(
        blocks
            .iter()
            .any(|option| option.blocker == strix && option.attackers.contains(&pirate)),
        "the Strix is offered as a blocker for the Pirate: {blocks:?}"
    );
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        unreachable!("attack_and_collect_blocks stops on nothing else")
    };
    assert_eq!(player, p1, "the defending seat declares the blocks");
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(strix, pirate)],
            },
        )
        .expect("the pairing came out of the list the engine published");

    // Combat damage is two steps (CR 510.4), so the walk is taken past both of
    // them: an empty stack is already true the instant blockers are declared,
    // and the end step is the first place the board can be read after damage.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, baleful_strix()).is_some(),
        "four first strike damage is lethal to a 1/1 (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p0, ramirez_de_pietro()).is_some(),
        "and the Strix never got a damage step to spend its deathtouch in: \
         simultaneous damage would have killed a creature with 3 toughness"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the attack was blocked, so no damage reached the player it was aimed at"
    );
}
