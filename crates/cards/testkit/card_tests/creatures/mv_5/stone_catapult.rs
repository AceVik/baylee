//! `cards/creatures/mv_5/stone_catapult.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn stone_catapult() -> CardIndex {
    card_index("c3ead850-3f22-4edb-ac96-40abbdf27905")
}

fn wei_elite_companions() -> CardIndex {
    card_index("2db5e0cd-c991-4fda-9001-9e4f10e8a805")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Stone Catapult (c3ead850-3f22-4edb-ac96-40abbdf27905) — `{4}{B}` 1/2
/// Creature — Human Soldier:
/// "`{T}`: Destroy target tapped nonblack creature. Activate only during your turn,
/// before attackers are declared."
#[test]
fn stone_catapult_destroys_only_tapped_nonblack_creature_before_attackers() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears(), wei_elite_companions(), hill_giant()])
        .hand(0, &[stone_catapult()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, stone_catapult());
    pass_until(&mut engine, stack_is_empty);

    let catapult = on_battlefield(&engine, P0, stone_catapult()).expect("Stone Catapult entered");
    assert_eq!(pt(&engine, catapult), (1, 2));

    // P1's turn: Bears (green) and Wei Elite Companions (black) attack and become tapped;
    // Hill Giant stays untapped. Catapult cannot activate on P1's turn.
    reach_their_main_phase(&mut engine, P1);
    let tapped_green = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let tapped_black = on_battlefield(&engine, P1, wei_elite_companions()).unwrap();
    let untapped_red = on_battlefield(&engine, P1, hill_giant()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (tapped_green, Defender::Player(P0)),
                    (tapped_black, Defender::Player(P0)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(!ability_offered(&engine, catapult, 0));

    // P0's turn before attackers: only tapped nonblack creature is offered.
    reach_their_main_phase(&mut engine, P0);
    activate(&mut engine, P0, stone_catapult(), 0);
    let options = pass_until_targets(&mut engine, P0);
    assert_eq!(
        options,
        vec![tapped_green],
        "tapped black creature and untapped nonblack creature must both be excluded"
    );
    let _ = (tapped_black, untapped_red);

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![tapped_green],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, P1, wei_elite_companions()).is_some());
    assert!(on_battlefield(&engine, P1, hill_giant()).is_some());
}
