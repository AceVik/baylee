//! Oracle scenarios for Fervent Charge.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn fervent_charge() -> CardIndex {
    card_index("6226db2b-d1a8-4c41-b802-67d3f65d2ca3")
}

#[test]
fn fervent_charge_pumps_attacking_creature_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(
            0,
            &[
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::PLAINS,
                index::SWAMP,
                index::GRIZZLY_BEARS,
                index::LLANOWAR_ELVES,
            ],
        )
        .hand(0, &[fervent_charge()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&e, p0, index::LLANOWAR_ELVES).unwrap();

    cast_from_hand(&mut e, p0, fervent_charge());
    pass_until(&mut e, stack_is_empty);

    attack_with(&mut e, bears, p1);
    assert!(
        !stack_is_empty(&e),
        "Fervent Charge attack trigger is on the stack"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, bears), (4, 4), "attacking creature gets +2/+2");
    assert_eq!(pt(&e, elf), (1, 1), "non-attacking creature is unchanged");

    reach_their_main_phase(&mut e, p1);
    assert_eq!(life_of(&e, p1), 16, "4 combat damage dealt to defender");
    assert_eq!(pt(&e, bears), (2, 2), "+2/+2 wore off at end of turn");
}

#[test]
fn fervent_charge_does_not_trigger_for_opponent_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[fervent_charge()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);

    let p1_bears = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    attack_with(&mut e, p1_bears, p0);
    assert!(stack_is_empty(&e), "no trigger for opponent's attacker");
    assert_eq!(pt(&e, p1_bears), (2, 2));
}
