//! `cards/creatures/mv_5/zhang_he_wei_general.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn zhang_he_wei_general() -> CardIndex {
    card_index("fe35b809-7439-4bdc-bce2-42262095064f")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Zhang He, Wei General (fe35b809-7439-4bdc-bce2-42262095064f) — `{3}{B}{B}` 4/2
/// Legendary Creature — Human Soldier:
/// - Horsemanship
/// - Whenever this creature attacks, each other creature you control gets +1/+0 until end of turn.
#[test]
fn zhang_he_wei_general_pumps_only_other_creatures_you_control_when_attacking() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                grizzly_bears(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .hand(0, &[zhang_he_wei_general()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, zhang_he_wei_general());
    pass_until(&mut engine, stack_is_empty);

    let zhang_he = on_battlefield(&engine, P0, zhang_he_wei_general()).expect("Zhang He entered");
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let elf = on_battlefield(&engine, P0, llanowar_elves()).unwrap();
    let opp_giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    assert_eq!(pt(&engine, zhang_he), (4, 2));
    assert!(keywords(&engine, zhang_he).contains(KeywordSet::HORSEMANSHIP));

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    // Attack with Zhang He and Bears while Llanowar Elves stays back.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (zhang_he, Defender::Player(P1)),
                    (bears, Defender::Player(P1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert_eq!(
        pt(&engine, zhang_he),
        (4, 2),
        "Zhang He does not pump himself"
    );
    assert_eq!(pt(&engine, bears), (3, 2), "attacking ally gets +1/+0");
    assert_eq!(
        pt(&engine, elf),
        (2, 1),
        "non-attacking ally also gets +1/+0"
    );
    assert_eq!(
        pt(&engine, opp_giant),
        (3, 3),
        "opponent creature is unchanged"
    );

    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        13,
        "4 + 3 combat damage dealt"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(pt(&engine, elf), (1, 1));
}
