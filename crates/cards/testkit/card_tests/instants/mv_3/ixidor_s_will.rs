//! `cards/instants/mv_3/ixidor_s_will.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "a84c9507-d054-4770-84f3-5a4b5a01d826"
fn ixidor_s_will() -> CardIndex {
    card_index("a84c9507-d054-4770-84f3-5a4b5a01d826")
}

/// Ixidor's Will ({2}{U}): taxes {2} per Wizard on the battlefield across BOTH
/// sides (ignoring non-Wizards); declining counters the spell, paying lets it
/// resolve.
#[test]
fn ixidors_will_taxes_two_per_wizard_on_battlefield_and_counters_if_unpaid() {
    // 1 Wizard on P0 + 1 Wizard on P1 + 1 non-Wizard Bear => 2 Wizards => tax {4}.
    let mut declined = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), index::PRODIGAL_SORCERER])
        .hand(0, &[ixidor_s_will()])
        .battlefield(
            1,
            &[mountain(), index::PRODIGAL_SORCERER, index::GRIZZLY_BEARS],
        )
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut declined);
    reach_their_main_phase(&mut declined, P1);

    tap_all_mana(&mut declined, P1);
    cast_with_floating(&mut declined, P1, lightning_bolt());
    lk::aim(&mut declined, &[], &[P0]);
    let bolt = on_stack(&declined, lightning_bolt()).unwrap();
    declined.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut declined, P0);
    cast_with_floating(&mut declined, P0, ixidor_s_will());
    lk::aim(&mut declined, &[bolt], &[]);
    pass_until(&mut declined, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(matches!(
        declined.pending(),
        Pending::YesNo { player, prompt: YesNoPrompt::PayTax { mana: 4 }, .. } if *player == P1
    ));
    declined.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut declined, stack_is_empty);

    assert_eq!(lk::life(&declined, P0), 20, "unpaid tax countered Bolt");

    // Paying the {4} tax lets Bolt resolve.
    let mut paid = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), index::PRODIGAL_SORCERER])
        .hand(0, &[ixidor_s_will()])
        .battlefield(
            1,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                index::PRODIGAL_SORCERER,
            ],
        )
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut paid);
    reach_their_main_phase(&mut paid, P1);

    tap_all_mana(&mut paid, P1);
    cast_with_floating(&mut paid, P1, lightning_bolt());
    lk::aim(&mut paid, &[], &[P0]);
    let bolt = on_stack(&paid, lightning_bolt()).unwrap();
    paid.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut paid, P0);
    cast_with_floating(&mut paid, P0, ixidor_s_will());
    lk::aim(&mut paid, &[bolt], &[]);
    pass_until(&mut paid, |e| matches!(e.pending(), Pending::YesNo { .. }));
    paid.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut paid, stack_is_empty);

    assert_eq!(lk::pool_total(&paid, P1), 0, "P1 spent {{4}} tax");
    assert_eq!(lk::life(&paid, P0), 17, "Bolt resolved for 3 damage");
}
