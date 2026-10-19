//! `cards/instants/mv_3/morale.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn morale() -> CardIndex {
    card_index("8af9d623-efa3-4cbc-a903-aaec32d78b0a")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
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

/// Morale (8af9d623-efa3-4cbc-a903-aaec32d78b0a) — "Attacking creatures get
/// +1/+1 until end of turn."
///
/// Cast with a creature attacking and another one at home: only the
/// attacker grows, and it stops at end of turn.
#[test]
fn morale_pumps_attackers_and_not_creatures_that_stayed_home() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                grizzly_bears(),
                savannah_lions(),
            ],
        )
        .hand(0, &[morale()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions are out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant is out");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .expect("the Bears attack");

    cast_from_hand(&mut engine, p0, morale());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (3, 3), "the attacker grows");
    assert_eq!(pt(&engine, lions), (2, 1), "the Lions stayed home");
    assert_eq!(pt(&engine, giant), (3, 3), "and so did the Giant");

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(pt(&engine, bears), (2, 2), "until end of turn");
}

/// "Attacking creatures" is no friendlier to its caster than that: an
/// opponent's attacker grows too.
#[test]
fn morale_also_pumps_an_opponents_attacker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), grizzly_bears()])
        .hand(0, &[morale()])
        .battlefield(1, &[obsianus_golem()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let golem = on_battlefield(&engine, p1, obsianus_golem()).expect("the Golem is out");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");

    attack_and_collect_blocks(&mut engine, golem, p0);
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("p0 does not block");
    priority_for(&mut engine, p0);
    assert_eq!(pt(&engine, golem), (4, 6));

    cast_from_hand(&mut engine, p0, morale());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, golem), (5, 7), "the attacking Golem grows");
    assert_eq!(pt(&engine, bears), (2, 2), "my idle Bears do not");
}
