//! `cards/creatures/mv_2/varchild_s_war_riders.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn varchild_s_war_riders_upkeep_gives_opponent_survivors_and_rampage_pumps_when_gang_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("258921f2-df26-4d8a-ad34-8f44d0158ae7");
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

fn age_counters(engine: &Engine<RegistryLookup>, id: ObjectId) -> u32 {
    engine
        .state()
        .object(id)
        .unwrap()
        .counters
        .get(baylee_cards_dsl::counters::AGE)
        .into()
}

#[test]
fn varchild_s_war_riders_price_grows_on_its_controllers_upkeep_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("258921f2-df26-4d8a-ad34-8f44d0158ae7");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[card, mountain()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    // Turn 1, p0's upkeep: age 1, one Survivor for the opponent, none for p0.
    reach_main_phase(&mut engine, p0);
    let riders = on_battlefield(&engine, p0, card).unwrap();
    assert_eq!(age_counters(&engine, riders), 1);
    assert_eq!(tokens_of(&engine, p1).len(), 1);
    assert!(tokens_of(&engine, p0).is_empty());
    // The opponent's upkeep is not "your upkeep": nothing more happens.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(age_counters(&engine, riders), 1);
    assert_eq!(tokens_of(&engine, p1).len(), 1);
    // p0's next upkeep: age 2, so two more Survivors (one per age counter).
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(age_counters(&engine, riders), 2);
    assert_eq!(
        tokens_of(&engine, p1).len(),
        3,
        "1 from the first upkeep and 2 from the second"
    );
    assert!(tokens_of(&engine, p0).is_empty());
}

#[test]
fn varchild_s_war_riders_is_not_pumped_by_a_single_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("258921f2-df26-4d8a-ad34-8f44d0158ae7");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[card, mountain()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let riders = on_battlefield(&engine, p0, card).unwrap();
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
                blockers: vec![(bear, riders)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, riders),
        (3, 4),
        "rampage 1 counts only blockers beyond the first"
    );
}
