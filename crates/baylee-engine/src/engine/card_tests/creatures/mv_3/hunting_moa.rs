//! `cards/creatures/mv_3/hunting_moa.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn hunting_moa() -> CardIndex {
    card_index("102d73bc-94ed-4a57-9872-6f7b9bf282f9")
}

/// Hunting Moa (102d73bc-94ed-4a57-9872-6f7b9bf282f9) — "Echo {2}{G} / When
/// this creature enters or dies, put a +1/+1 counter on target creature."
///
/// A noncreature artifact is not offered as a target; entering puts a +1/+1
/// counter on the chosen creature, and declining Echo at the next upkeep
/// sacrifices Hunting Moa and puts a second +1/+1 counter on the chosen
/// creature.
#[test]
fn hunting_moa_puts_plus_one_counter_on_etb_and_on_dies_when_echo_declined() {
    let mut e = lk::game(
        SEED,
        &[llanowar_elves(), sol_ring()],
        &[],
        &[hunting_moa()],
        &[],
    );
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let ring = lk::obj(&e, lk::P0, sol_ring());

    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 3)]);
    cast_with_floating(&mut e, lk::P0, hunting_moa());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected target choice");
    };
    assert!(options.contains(&elf), "creature is a legal target");
    assert!(
        !options.contains(&ring),
        "noncreature artifact is not a legal target"
    );
    lk::aim(&mut e, &[elf], &[]);
    lk::settle(&mut e);

    let moa = lk::obj(&e, lk::P0, hunting_moa());
    assert_eq!(pt(&e, moa), (3, 2));
    assert_eq!(pt(&e, elf), (2, 2), "ETB put a +1/+1 counter on the Elf");

    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 3)]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[elf], &[]);
    lk::settle(&mut e);

    assert!(in_graveyard(&e, lk::P0, hunting_moa()).is_some());
    assert_eq!(
        pt(&e, elf),
        (3, 3),
        "dies trigger put a second +1/+1 counter on the Elf"
    );
}

/// Hunting Moa — paying Echo {2}{G} at the next upkeep keeps it on the
/// battlefield.
#[test]
fn hunting_moa_survives_when_echo_is_paid() {
    let mut e = lk::game(SEED, &[], &[], &[hunting_moa()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 3)]);
    cast_with_floating(&mut e, lk::P0, hunting_moa());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let moa = lk::obj(&e, lk::P0, hunting_moa());
    lk::aim(&mut e, &[moa], &[]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, moa), (4, 3));

    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 3)]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    reach_main_phase(&mut e, lk::P0);

    assert!(on_battlefield(&e, lk::P0, hunting_moa()).is_some());
    assert_eq!(pt(&e, moa), (4, 3));
}
