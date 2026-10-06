//! `cards/creatures/mv_4/juzam_djinn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "4e81596c-9225-43d1-bd35-798212144f2c"

/// Juzám Djinn is `{2}{B}{B}` for a 5/5 whose entire text is one sentence:
/// "At the beginning of your upkeep, this creature deals 1 damage to you."
///
/// Both halves of that have to be played rather than read. Four Swamps pay the
/// printed cost down to an empty pool and leave the body on the table, and then
/// a turn boundary has to be crossed before the trigger exists at all — which
/// is why the damage is read on the far side of the upkeep, at p0's *next*
/// first main phase, where the one missing life can only be the printed
/// sentence. The opponent's untouched twenty is the control: a trigger that had
/// lost its `PlayerRel::You` would have shown up on the other side of the table.
#[test]
fn juzam_djinn_bites_its_own_controller_for_one_on_each_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[juzam_djinn()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // `{2}{B}{B}` out of exactly four Swamps, so the pool empties on the cast
    // and the life total is the only thing left on this board that can move.
    cast_from_hand(&mut engine, p0, juzam_djinn());
    pass_until(&mut engine, stack_is_empty);
    let djinn = on_battlefield(&engine, p0, juzam_djinn()).expect("the Djinn resolved");
    assert_eq!(pt(&engine, djinn), (5, 5), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}}{{B}}{{B}} is the whole of the four Swamps"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "no upkeep has come round yet, so nothing has bitten its controller"
    );

    // Round both seats to p0's next first main phase, which is the far side of
    // the upkeep the trigger belongs to. The only questions on the way are the
    // combat declarations the walk passes through and the trigger's own target.
    for _ in 0..400 {
        if engine.state().turn.active == p0
            && matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().players[0].life < 20
        {
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
                player_options,
                ..
            } => {
                assert_eq!(player, p0, "\"to you\" is the Djinn's controller");
                assert!(
                    player_options.contains(&p0),
                    "the controller is one of the seats the sentence may name: {player_options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p0],
                        },
                    )
                    .unwrap();
            }
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, p0, "\"to you\" is the Djinn's controller");
                assert!(
                    options.contains(&p0),
                    "the controller is one of the seats the sentence may name: {options:?}"
                );
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p0))
                    .unwrap();
            }
            other => panic!("unexpected on the way to the next upkeep: {other:?}"),
        }
    }

    assert!(
        engine.state().turn.active == p0 && matches!(engine.state().turn.phase, Phase::FirstMain),
        "the walk reaches p0's next first main phase, got {:?} on turn {}",
        engine.state().turn.phase,
        engine.state().turn.number
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"deals 1 damage to you\" — one, once, and not one for every turn that passed"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage belongs to the controller, not across the table"
    );
    assert!(
        on_battlefield(&engine, p0, juzam_djinn()).is_some(),
        "the Djinn is still standing, so the life was lost to its own sentence \
         and not to a creature that died"
    );
}
