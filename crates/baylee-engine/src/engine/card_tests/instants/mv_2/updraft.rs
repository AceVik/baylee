//! Updraft — `{1}{U}` Instant (Ice Age)
//!
//! Target creature gains flying until end of turn.
//! Draw a card at the beginning of the next turn's upkeep.

#[allow(clippy::wildcard_imports)]
use super::*;

fn updraft() -> CardIndex {
    card_index("ff0d5cbe-99d3-4d3b-9a5b-80223fc18869")
}

#[test]
fn updraft_grants_flying_until_end_of_turn_and_draws_on_next_turns_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[updraft()])
        .battlefield(0, &[island(), island(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attacker = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let spell = in_hand(&engine, p0, updraft()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![attacker],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Grizzly Bears now has flying, so opponent's ground Grizzly Bears cannot block it.
    let options = attack_and_collect_blocks(&mut engine, attacker, p1);
    assert!(
        options.iter().all(|o| !o.attackers.contains(&attacker)),
        "ground Grizzly Bears cannot block a creature granted flying by Updraft"
    );
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Advance to p1's upkeep: flying wore off at EOT, and p0 draws 1 card.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !engine
            .state()
            .object(attacker)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING),
        "Updraft's flying wore off at end of turn"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "Updraft draws a card at the next turn's upkeep"
    );
}
