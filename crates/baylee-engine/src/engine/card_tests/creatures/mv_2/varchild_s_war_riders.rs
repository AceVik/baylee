//! `cards/creatures/mv_2/varchild_s_war_riders.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const VARCHILD_S_WAR_RIDERS: &str = "258921f2-df26-4d8a-ad34-8f44d0158ae7";

#[test]
fn varchild_s_war_riders_upkeep_gives_opponent_survivors_and_rampage_pumps_when_gang_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index(VARCHILD_S_WAR_RIDERS);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[card, mountain()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    // Turn 1 upkeep triggers cumulative upkeep: puts 1 age counter on Varchild's War-Riders
    // and creates one 1/1 red Survivor token for p1.
    reach_main_phase(&mut engine, p0);

    let riders = on_battlefield(&engine, p0, card).unwrap();
    assert_eq!(
        engine
            .state()
            .object(riders)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );

    let survivor = engine
        .state()
        .battlefield_view()
        .into_iter()
        .find(|&id| {
            let obj = engine.state().object(id).unwrap();
            obj.controller == p1 && obj.token.is_some()
        })
        .expect("p1 should have a 1/1 Survivor token");
    assert_eq!(pt(&engine, survivor), (1, 1));

    // Attack with Varchild's War-Riders; p1 blocks with both Grizzly Bears and the Survivor.
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(riders, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, riders), (survivor, riders)],
            },
        )
        .unwrap();

    // Rampage 1 triggers and gives +1/+1 for the 1 blocker beyond the first -> 4/5.
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, riders), (4, 5));
}
