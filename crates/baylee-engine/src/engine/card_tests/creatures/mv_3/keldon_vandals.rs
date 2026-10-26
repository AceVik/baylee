//! `cards/creatures/mv_3/keldon_vandals.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn keldon_vandals() -> CardIndex {
    card_index("0aa7bd6a-0a8d-4413-a9e8-b7a459300841")
}

/// Keldon Vandals (0aa7bd6a-0a8d-4413-a9e8-b7a459300841) — "Echo {2}{R} /
/// When this creature enters, destroy target artifact."
///
/// Non-artifacts are excluded from the target offer; the targeted artifact is
/// destroyed on ETB; Echo sacrifices Keldon Vandals if declined and keeps it
/// if paid.
#[test]
fn keldon_vandals_destroys_target_artifact_on_etb_and_has_echo() {
    let mut e = lk::game(
        SEED,
        &[],
        &[sol_ring(), llanowar_elves()],
        &[keldon_vandals()],
        &[],
    );
    let their_ring = lk::obj(&e, lk::P1, sol_ring());
    let their_elf = lk::obj(&e, lk::P1, llanowar_elves());

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 3)]);
    cast_with_floating(&mut e, lk::P0, keldon_vandals());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected target choice");
    };
    assert!(options.contains(&their_ring));
    assert!(
        !options.contains(&their_elf),
        "non-artifact creature is not a legal target"
    );
    lk::aim(&mut e, &[their_ring], &[]);
    lk::settle(&mut e);

    assert!(in_graveyard(&e, lk::P1, sol_ring()).is_some());
    assert!(on_battlefield(&e, lk::P0, keldon_vandals()).is_some());

    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 3)]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, lk::P0, keldon_vandals()).is_some());
}

#[test]
fn keldon_vandals_survives_when_echo_is_paid() {
    let mut e = lk::game(SEED, &[], &[sol_ring()], &[keldon_vandals()], &[]);
    let their_ring = lk::obj(&e, lk::P1, sol_ring());

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 3)]);
    cast_with_floating(&mut e, lk::P0, keldon_vandals());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[their_ring], &[]);
    lk::settle(&mut e);

    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 3)]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    reach_main_phase(&mut e, lk::P0);

    assert!(on_battlefield(&e, lk::P0, keldon_vandals()).is_some());
}
