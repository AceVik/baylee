//! `cards/creatures/mv_4/highway_robber.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Highway Robber is a {2}{B}{B} 2/2 whose enters trigger reads "target
/// opponent loses 2 life and you gain 2 life", and both halves of that
/// sentence are the engine's answer rather than the card's. The target is an
/// *opponent*, so the menu the drain is aimed at is read from both sides of
/// the table — it holds the seat across it and never the seat that cast the
/// creature — and the drain is read as one life total moving each way, which
/// a card that had said "target player" or paid the wrong seat could not show.
/// Four Swamps are tapped into the pool before the cast, so the creature
/// really arrives and the trigger really fires.
#[test]
fn highway_robber_drains_the_opponent_it_names_and_pays_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(41, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[highway_robber()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has drained yet"
    );
    assert_eq!(engine.state().players[1].life, 20, "on either side");

    // Four Swamps into the pool first: {2}{B}{B} is the whole cost, and both
    // the cast and the offer behind it are read off the pool.
    cast_from_hand(&mut engine, p0, highway_robber());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the four Swamps paid the {{2}}{{B}}{{B}}"
    );

    // The spell resolves and its enters trigger is put on the stack, which is
    // where its target is named (CR 603.3d). The two arms are the same choice
    // arriving as a player-or-object prompt and as a player-only one; nothing
    // else may be asked before it, so anything else is a finding.
    let mut asked = false;
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert_eq!(player, p0, "the seat that cast it aims the trigger");
                assert_eq!(
                    player_options,
                    vec![p1],
                    "\"target opponent\" reaches the seat across the table and \
                     never its own controller"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .expect("the opponent was one of the options it enumerated");
                asked = true;
                break;
            }
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, p0, "the seat that cast it aims the trigger");
                assert_eq!(
                    options,
                    vec![p1],
                    "\"target opponent\" reaches the seat across the table and \
                     never its own controller"
                );
                engine
                    .apply(p0, PlayerAction::ChoosePlayer(p1))
                    .expect("the opponent was one of the options it enumerated");
                asked = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!(
                "the enters trigger is put on the stack before anything else can \
                 happen: {other:?}"
            ),
        }
    }
    assert!(asked, "the trigger asks which opponent loses the two life");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "\"target opponent loses 2 life\" — the seat that was named, and two"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "\"and you gain 2 life\" — the gain belongs to the controller, not to \
         the seat that lost"
    );
    let robber = on_battlefield(&engine, p0, highway_robber())
        .expect("the 2/2 itself resolved onto the battlefield");
    assert_eq!(pt(&engine, robber), (2, 2), "the body the card prints");
}
