//! `cards/creatures/mv_3/tolarian_drake.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::object::Status;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn tolarian_drake() -> CardIndex {
    card_index("d999f2a3-5cb4-4f12-a79a-c5db0a300620")
}

fn phased_out(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    e.state()
        .object(id)
        .expect("the object exists")
        .status
        .contains(Status::PHASED_OUT)
}

/// Tolarian Drake (d999f2a3-5cb4-4f12-a79a-c5db0a300620) — "Flying. Phasing:
/// this phases in or out before you untap during each of your untap steps." A
/// 2/4 flyer with phasing: in play on the opponent's turn 2 (their untap steps
/// do not phase it), phased out from its controller's untap step on turn 3, still
/// out on turn 4, and phased back in on turn 5.
#[test]
fn tolarian_drake_phases_at_its_controllers_untap_steps_only() {
    let mut e = lk::game(
        SEED,
        &[island(), island(), island()],
        &[],
        &[tolarian_drake()],
        &[],
    );
    cast_from_hand(&mut e, P0, tolarian_drake());
    pass_until(&mut e, stack_is_empty);
    let drake = lk::obj(&e, P0, tolarian_drake());
    assert_eq!(pt(&e, drake), (2, 4));
    assert!(keywords(&e, drake).contains(KeywordSet::FLYING));
    assert!(keywords(&e, drake).contains(KeywordSet::PHASING));

    reach_their_main_phase(&mut e, P1);
    assert!(
        !phased_out(&e, drake),
        "the opponent's untap step does not phase it"
    );

    reach_their_main_phase(&mut e, P0);
    assert!(
        phased_out(&e, drake),
        "phased out at its controller's untap step (turn 3)"
    );

    reach_their_main_phase(&mut e, P1);
    assert!(phased_out(&e, drake), "still out on the opponent's turn 4");

    reach_their_main_phase(&mut e, P0);
    assert!(
        !phased_out(&e, drake),
        "phased in again at its controller's next untap step"
    );
}
