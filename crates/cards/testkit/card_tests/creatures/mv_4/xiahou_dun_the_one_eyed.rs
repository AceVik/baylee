//! `cards/creatures/mv_4/xiahou_dun_the_one_eyed.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn xiahou_dun_the_one_eyed() -> CardIndex {
    card_index("9602f262-62cb-4c1e-91b6-d94780e571cf")
}

fn murder() -> CardIndex {
    card_index("938b4e2c-88d9-4637-bc00-e228920c9a78")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn wei_elite_companions() -> CardIndex {
    card_index("2db5e0cd-c991-4fda-9001-9e4f10e8a805")
}

/// Xiahou Dun, the One-Eyed (9602f262-62cb-4c1e-91b6-d94780e571cf) — `{2}{B}{B}` 3/2
/// Legendary Creature — Human Soldier:
/// - Horsemanship
/// - Sacrifice this creature: Return target black card from your graveyard to your hand.
///   Activate only during your turn, before attackers are declared.
#[test]
fn xiahou_dun_sacrifices_to_return_any_black_card_from_your_graveyard_to_hand() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[festering_goblin()])
        .hand(0, &[xiahou_dun_the_one_eyed(), murder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let opp_goblin = on_battlefield(&engine, P1, festering_goblin()).unwrap();
    let own_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    bury(&mut engine, &[own_bears]);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, murder());
    aim_at(&mut engine, P0, opp_goblin);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, xiahou_dun_the_one_eyed());
    pass_until(&mut engine, stack_is_empty);

    let xh = on_battlefield(&engine, P0, xiahou_dun_the_one_eyed()).expect("Xiahou Dun entered");
    assert_eq!(pt(&engine, xh), (3, 2));
    assert!(keywords(&engine, xh).contains(KeywordSet::HORSEMANSHIP));

    let own_black = in_graveyard(&engine, P0, murder()).unwrap();
    let own_green = in_graveyard(&engine, P0, grizzly_bears()).unwrap();
    let opp_black = in_graveyard(&engine, P1, festering_goblin()).unwrap();

    activate(&mut engine, P0, xiahou_dun_the_one_eyed(), 0);
    let options = pass_until_targets(&mut engine, P0);
    assert!(
        options.contains(&own_black),
        "black noncreature card in your yard is legal"
    );
    assert!(!options.contains(&own_green), "nonblack card is excluded");
    assert!(
        !options.contains(&opp_black),
        "opponent's graveyard is excluded"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![own_black],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, xiahou_dun_the_one_eyed()).is_some());
    assert!(in_hand(&engine, P0, murder()).is_some());
}

#[test]
fn xiahou_dun_respects_timing_and_cannot_target_himself_before_sacrifice_cost() {
    let mut empty_yard = Duel::new(SEED, swamp())
        .battlefield(0, &[xiahou_dun_the_one_eyed()])
        .start();
    keep_mulligans(&mut empty_yard);
    reach_main_phase(&mut empty_yard, P0);
    let xh_empty = on_battlefield(&empty_yard, P0, xiahou_dun_the_one_eyed()).unwrap();
    assert!(
        !ability_offered(&empty_yard, xh_empty, 0),
        "CR 602.2b: target is chosen before paying SacrificeSelf, so empty yard has no target"
    );

    let mut timing = Duel::new(SEED, swamp())
        .battlefield(0, &[xiahou_dun_the_one_eyed(), wei_elite_companions()])
        .start();
    keep_mulligans(&mut timing);
    let companions = on_battlefield(&timing, P0, wei_elite_companions()).unwrap();
    bury(&mut timing, &[companions]);
    reach_main_phase(&mut timing, P0);
    let xh = on_battlefield(&timing, P0, xiahou_dun_the_one_eyed()).unwrap();
    assert!(ability_offered(&timing, xh, 0));

    pass_until(&mut timing, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    timing
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    pass_until(&mut timing, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        !ability_offered(&timing, xh, 0),
        "cannot activate in second main phase"
    );

    reach_their_main_phase(&mut timing, P1);
    assert!(
        !ability_offered(&timing, xh, 0),
        "cannot activate on opponent's turn"
    );
}
