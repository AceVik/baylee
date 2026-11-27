//! `cards/sorceries/mv_4/stench_of_evil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn stench_of_evil() -> CardIndex {
    card_index("14062abe-14df-4dbe-ad7b-530e5f6c4988")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

#[test]
fn stench_of_evil_destroys_all_plains_and_taxes_or_damages_per_land() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), plains()])
        .battlefield(1, &[plains(), plains(), forest()])
        .hand(0, &[stench_of_evil()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    // P1 has 2 floating mana to pay for 1 of the 2 Plains, and declines/cant pay the other.
    kit::float(&mut e, P1, &[(ManaColor::Colorless, 2)]);
    cast_from_hand(&mut e, P0, stench_of_evil());

    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    // First controller in APNAP or P0 first: P0 declines (takes 1), P1 pays for one and declines the other.
    let mut seen_p1 = 0;
    while !stack_is_empty(&e) {
        if let Pending::YesNo { player, .. } = e.pending().clone() {
            if player == P0 {
                e.apply(P0, PlayerAction::YesNo(false)).unwrap();
            } else {
                e.apply(P1, PlayerAction::YesNo(seen_p1 == 0)).unwrap();
                seen_p1 += 1;
            }
        } else {
            let Pending::Priority { player, .. } = e.pending().clone() else {
                unreachable!("{:?}", e.pending());
            };
            e.apply(player, PlayerAction::PassPriority).unwrap();
        }
    }

    assert!(on_battlefield(&e, P0, plains()).is_none());
    assert!(on_battlefield(&e, P1, plains()).is_none());
    assert!(
        on_battlefield(&e, P1, forest()).is_some(),
        "non-Plains untouched"
    );
    assert_eq!(e.state().players[0].life, 19);
    assert_eq!(e.state().players[1].life, 19);
}
