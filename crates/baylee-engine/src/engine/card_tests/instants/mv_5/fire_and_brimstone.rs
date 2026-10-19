//! `cards/instants/mv_5/fire_and_brimstone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn fire_and_brimstone() -> CardIndex {
    card_index("b1f857c5-cd22-4a4e-8039-1e7eeb3754d8")
}

fn obsianus_golem() -> CardIndex {
    card_index("ac41171e-c454-49e9-9004-c082ae099630")
}

/// Passes priority until `seat` holds it.
#[track_caller]
fn priority_for(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } if player == seat => return,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("expected priority on the way to {seat:?}, got {other:?}"),
        }
    }
    panic!("{seat:?} never got priority");
}

/// Fire and Brimstone (b1f857c5-cd22-4a4e-8039-1e7eeb3754d8) — "Fire and
/// Brimstone deals 4 damage to target player who attacked this turn and 4
/// damage to you."
///
/// The opponent attacks; the only legal target is the player who attacked,
/// and both of us lose 4.
#[test]
fn fire_and_brimstone_hits_the_player_who_attacked_and_the_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[fire_and_brimstone()])
        .battlefield(1, &[obsianus_golem()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let golem = on_battlefield(&engine, p1, obsianus_golem()).expect("the Golem is out");

    attack_and_collect_blocks(&mut engine, golem, p0);
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("p0 has nothing to block with");
    priority_for(&mut engine, p0);

    cast_from_hand(&mut engine, p0, fire_and_brimstone());
    match engine.pending().clone() {
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0);
            assert_eq!(options, vec![p1], "only the player who attacked");
            engine
                .apply(p0, PlayerAction::ChoosePlayer(p1))
                .expect("the attacker is a legal target");
        }
        Pending::ChooseTargets {
            player,
            options,
            player_options,
            ..
        } => {
            assert_eq!(player, p0);
            assert!(options.is_empty(), "no object is a legal target");
            assert_eq!(player_options, vec![p1], "only the player who attacked");
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .expect("the attacker is a legal target");
        }
        other => panic!("expected a player to be named, got {other:?}"),
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 16, "4 to the attacker");
    assert_eq!(engine.state().players[0].life, 16, "and 4 to me");
}

/// With nobody having attacked there is nobody to aim it at.
#[test]
fn fire_and_brimstone_cannot_be_cast_before_anyone_attacks() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[fire_and_brimstone()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    tap_all_mana(&mut engine, p0);

    let card = in_hand(&engine, p0, fire_and_brimstone()).expect("it is in hand");
    assert!(
        engine.apply(p0, PlayerAction::CastSpell { card }).is_err(),
        "no player has attacked this turn"
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}
