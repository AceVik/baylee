//! `cards/enchantments/mv_2/invigorating_boon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn invigorating_boon() -> CardIndex {
    card_index("d00329c2-e65c-4774-83cd-d68e1d1a6e31")
}

fn tranquil_thicket() -> CardIndex {
    card_index("9f8fe514-77ed-41b4-a6f3-c6f095bb97be")
}

fn putrid_imp() -> CardIndex {
    card_index("7192ff07-9a84-49ad-a40c-ce255ce21544")
}

/// Invigorating Boon triggers when either player cycles a card (offering a
/// voluntary +1/+1 counter on target creature, which may also be declined), and
/// ignores plain non-cycling discards.
#[test]
fn invigorating_boon_triggers_on_either_players_cycle_not_plain_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                invigorating_boon(),
                llanowar_elves(),
                putrid_imp(),
            ],
        )
        .battlefield(1, &[forest()])
        .hand(0, &[tranquil_thicket(), tranquil_thicket(), swamp()])
        .hand(1, &[tranquil_thicket()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf seated");
    assert_eq!(pt(&engine, elf), (1, 1));

    // Plain discard via Putrid Imp does NOT trigger Invigorating Boon.
    let swamp_in_hand = in_hand(&engine, p0, swamp()).expect("Swamp in hand");
    activate(&mut engine, p0, putrid_imp(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swamp_in_hand],
            },
        )
        .expect("discard Swamp to Putrid Imp");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, elf), (1, 1), "plain discard must not trigger");

    // 1. P0 cycles Tranquil Thicket and accepts the optional +1/+1 counter.
    let green = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| id == green[0]);
    activate(&mut engine, p0, tranquil_thicket(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, true);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, elf), (2, 2));

    // 2. P0 cycles a second Tranquil Thicket and declines the optional counter.
    tap_mana_where(&mut engine, p0, |id| id == green[1]);
    activate(&mut engine, p0, tranquil_thicket(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, elf), (2, 2), "declining MayDo adds no counter");

    // 3. Opponent P1 cycles on their turn -> still triggers P0's Invigorating Boon.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, tranquil_thicket(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, true);
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert_eq!(
        pt(&engine, elf),
        (3, 3),
        "opponent cycle triggers P0's Boon"
    );
}
