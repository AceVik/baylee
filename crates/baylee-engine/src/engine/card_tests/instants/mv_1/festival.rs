//! `cards/instants/mv_1/festival.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn festival() -> CardIndex {
    card_index("8e594801-4a8d-4dcd-b328-7eb3de65cf90")
}

fn obsianus_golem() -> CardIndex {
    card_index("ac41171e-c454-49e9-9004-c082ae099630")
}

/// A duel in which p0 holds a Festival and one Plains, and p1 has a Golem
/// that is free to attack on its first turn.
fn board() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains()])
        .hand(0, &[festival()])
        .battlefield(1, &[obsianus_golem()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    engine
}

/// Passes priority until p0 holds it in p1's `step`.
#[track_caller]
fn to_their_step(engine: &mut Engine<RegistryLookup>, step: crate::turn::Step) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == step
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
}

/// What p1 is offered to attack with on its turn: the attackers of its
/// declaration, or nothing at all when the engine never asks.
#[track_caller]
fn attackers_p1_may_declare(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut seen_their_turn = false;
    for _ in 0..120 {
        if engine.state().turn.active == p1 {
            seen_their_turn = true;
        } else if seen_their_turn {
            assert_eq!(engine.state().turn.active, p0);
            return Vec::new();
        }
        match engine.pending().clone() {
            Pending::ChooseAttackers {
                player, attackers, ..
            } if player == p1 => return attackers,
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to p1's attack: {other:?}"),
        }
    }
    panic!("p1's turn never reached its attack");
}

/// Festival (8e594801-4a8d-4dcd-b328-7eb3de65cf90) — "Cast this spell only
/// during an opponent's upkeep. Creatures can't attack this turn."
///
/// Cast in p1's upkeep, it leaves p1 with no creature to attack with.
#[test]
fn festival_in_an_opponents_upkeep_stops_every_attack_that_turn() {
    let p1 = PlayerId::new(1);
    let mut engine = board();
    let golem = on_battlefield(&engine, p1, obsianus_golem()).expect("the Golem is out");

    to_their_step(&mut engine, crate::turn::Step::Upkeep);
    cast_from_hand(&mut engine, PlayerId::new(0), festival());
    pass_until(&mut engine, stack_is_empty);

    let offered = attackers_p1_may_declare(&mut engine);
    assert!(
        !offered.contains(&golem),
        "creatures can't attack this turn: {offered:?}"
    );
}

/// The other side of the same board: without the Festival the Golem is
/// offered as an attacker, so the test above proves the spell and not an
/// engine that never asks.
#[test]
fn without_festival_the_opponents_golem_may_attack() {
    let p1 = PlayerId::new(1);
    let mut engine = board();
    let golem = on_battlefield(&engine, p1, obsianus_golem()).expect("the Golem is out");

    let offered = attackers_p1_may_declare(&mut engine);
    assert!(
        offered.contains(&golem),
        "the Golem may attack: {offered:?}"
    );
}

/// "Cast this spell only during an opponent's upkeep": not in the caster's
/// own main phase, not in the opponent's draw step.
#[test]
fn festival_cannot_be_cast_outside_an_opponents_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = board();
    let card = in_hand(&engine, p0, festival()).expect("Festival is in hand");
    assert!(
        engine.apply(p0, PlayerAction::CastSpell { card }).is_err(),
        "not in my own main phase"
    );

    to_their_step(&mut engine, crate::turn::Step::Draw);
    assert!(
        engine.apply(p0, PlayerAction::CastSpell { card }).is_err(),
        "not in the opponent's draw step"
    );
    assert!(
        in_hand(&engine, p0, festival()).is_some(),
        "and Festival is still in hand"
    );
}
