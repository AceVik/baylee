//! `cards/creatures/mv_3/serendib_efreet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Serendib Efreet — {2}{U} for a 3/4 flier with one drawback printed under
/// it: "At the beginning of your upkeep, this creature deals 1 damage to
/// you."
///
/// Both halves are played, because neither can be read off the other. The
/// three Islands pay for the cast and are spent doing it, so the pool is
/// empty and the only thing left on the board that can move a life total is
/// the trigger; the body and the keyword are read off the permanent the cast
/// left behind. The opponent is the control that fixes the word "you": p1 is
/// at 20 before the bite and still at 20 on their next main phase, which is
/// also where the walk on shows the bite belongs to the *controller's*
/// upkeep rather than to every upkeep at the table.
#[test]
fn serendib_efreet_flies_and_bites_its_controller_on_its_own_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(2024, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[serendib_efreet()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, serendib_efreet());
    pass_until(&mut engine, stack_is_empty);
    let efreet = on_battlefield(&engine, p0, serendib_efreet()).expect("the Efreet resolved");
    assert_eq!(pt(&engine, efreet), (3, 4), "the printed 3/4 body");
    assert!(
        keywords(&engine, efreet).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the upkeep this Efreet arrived after has already passed"
    );

    // The trigger fires on p0's own next upkeep, and it is read on the life
    // total rather than on the question, because a `DealDamage` aimed at a
    // fixed player may arrive as a target choice or as nothing at all. The
    // walk answers whatever comes and stops the moment a life point moves.
    let mut bitten = false;
    for _ in 0..300 {
        if engine.state().players[0].life < 20 {
            bitten = true;
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => {
                // "deals 1 damage to you" names a player, so the answer is the
                // Efreet's controller whichever of the two lists carries them.
                let players: Vec<PlayerId> = if player_options.contains(&p0) {
                    vec![p0]
                } else {
                    player_options.iter().copied().take(1).collect()
                };
                let objects = if players.is_empty() {
                    options.iter().copied().take(1).collect()
                } else {
                    Vec::new()
                };
                engine
                    .apply(player, PlayerAction::ChooseTargets { objects, players })
                    .unwrap();
            }
            other => panic!("unexpected on the way to the Efreet's upkeep: {other:?}"),
        }
    }
    assert!(
        bitten,
        "the printed upkeep trigger never took a life point off its controller"
    );
    assert_eq!(
        engine.state().turn.active,
        p0,
        "the bite is on the controller's own upkeep, got turn for {:?}",
        engine.state().turn.active
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the trigger says \"you\": the Efreet bites nobody but its controller"
    );

    // The control: a whole turn later, off p0's upkeep entirely, the life
    // totals have not moved again. That is what tells "at the beginning of
    // *your* upkeep" from a trigger that sweeps the table every turn.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().players[0].life,
        19,
        "one bite, and no more, on the way through a turn that is not its controller's"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the seat that never controlled the Efreet is never damaged by it"
    );
    assert!(
        on_battlefield(&engine, p0, serendib_efreet()).is_some(),
        "the Efreet outlives the life it takes"
    );
}
