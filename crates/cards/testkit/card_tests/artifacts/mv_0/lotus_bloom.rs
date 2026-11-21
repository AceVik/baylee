//! `cards/artifacts/mv_0/lotus_bloom.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn lotus_bloom() -> CardIndex {
    card_index("04cf02dc-f053-414e-87d8-1537f25bcbf4")
}

/// Lotus Bloom (04cf02dc-f053-414e-87d8-1537f25bcbf4) — (no mana cost) Artifact:
/// "Suspend 3—{0}"
/// "{T}, Sacrifice this artifact: Add three mana of any one color."
///
/// Uncastable directly from hand (no mana cost), suspends for {0} with 3 time
/// counters, counts down across three of your upkeeps to cast free onto the
/// battlefield, and sacrifices for 3 mana of any one chosen color.
#[test]
fn lotus_bloom_suspends_three_turns_then_sacrifices_for_three_mana_of_one_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[lotus_bloom()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bloom_card = in_hand(&engine, p0, lotus_bloom()).unwrap();
    let offer = priority_offer(&engine);
    assert!(
        !offer.castable.contains(&bloom_card),
        "Lotus Bloom has no mana cost and cannot be cast normally from hand"
    );
    assert!(
        offer.suspendable.contains(&bloom_card),
        "Suspend 3—{{0}} is offered on an empty board"
    );

    engine
        .apply(p0, PlayerAction::Suspend { card: bloom_card })
        .unwrap();
    assert_eq!(
        engine.state().object(bloom_card).unwrap().zone,
        Zone::Exile,
        "suspended Lotus Bloom enters exile"
    );
    assert_eq!(counters_on(&engine, bloom_card, CounterKind::Time), 3);

    // Turns 3, 5, and 7 are p0's next three upkeeps: on turn 7 the last time
    // counter comes off and Lotus Bloom is cast without paying its mana cost.
    for turn in [3, 5, 7] {
        pass_until(&mut engine, |e| e.state().turn.number >= turn);
    }
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, lotus_bloom()).is_some() && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().turn.number,
        7,
        "enters on the third upkeep after suspension"
    );

    let bloom = on_battlefield(&engine, p0, lotus_bloom()).unwrap();
    activate(&mut engine, p0, lotus_bloom(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 5, "all five colors offered");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    assert_eq!(
        engine.state().object(bloom).unwrap().zone,
        Zone::Graveyard,
        "Lotus Bloom sacrificed as cost"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        3,
        "added three mana of the chosen color"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
}
