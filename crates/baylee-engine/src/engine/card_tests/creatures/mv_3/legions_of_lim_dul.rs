//! `cards/creatures/mv_3/legions_of_lim_dul.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn legions_of_lim_dul() -> CardIndex {
    card_index("70b69803-1870-4441-954b-dba5b433fcb2")
}

fn snow_covered_swamp() -> CardIndex {
    card_index("d8239a86-7184-4005-ba1e-2dddcd756c47")
}

fn snow_covered_forest() -> CardIndex {
    card_index("5f0d3be8-e63e-4ade-ae58-6b0c14f2ce6d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Legions of Lim-Dûl (70b69803-1870-4441-954b-dba5b433fcb2) — {1}{B}{B} 2/3 Creature — Zombie:
/// "Snow swampwalk (This creature can't be blocked as long as defending player controls a snow Swamp.)"
#[test]
fn legions_of_lim_dul_can_be_blocked_without_a_snow_swamp_and_cannot_with_one() {
    // 1. Defending player controls a basic (non-snow) Swamp and a Snow-Covered Forest, plus a blocker:
    // neither is a snow Swamp, so Legions of Lim-Dûl CAN be blocked.
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[legions_of_lim_dul()])
        .battlefield(1, &[swamp(), snow_covered_forest(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    let legions = on_battlefield(&engine, P0, legions_of_lim_dul()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(legions, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, legions)],
            },
        )
        .expect("block is legal when defending player controls no snow Swamp");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[1].life,
        20,
        "blocked attacker dealt no player damage"
    );

    // 2. Defending player controls a Snow-Covered Swamp and an untapped blocker:
    // snow swampwalk makes Legions of Lim-Dûl unblockable.
    let mut engine2 = Duel::new(SEED, swamp())
        .battlefield(0, &[legions_of_lim_dul()])
        .battlefield(1, &[snow_covered_swamp(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine2);

    let legions2 = on_battlefield(&engine2, P0, legions_of_lim_dul()).unwrap();
    pass_until(&mut engine2, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine2
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(legions2, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine2, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine2.state().players[1].life,
        18,
        "snow swampwalk prevented P1's Grizzly Bears from blocking"
    );
}
