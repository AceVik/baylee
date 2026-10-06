//! `cards/sorceries/mv_8/ondu_inversion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ondu Inversion` // `Ondu Skyruins` (`Coverage::Implemented`): "Destroy all nonland
/// permanents. // This land enters tapped. {T}: Add {W}."
///
/// Under `Coverage::Implemented`, casting the front-face sorcery destroys all nonland permanents
/// across both battlefields via `Effect::DestroyAll` with `Filter::NONLAND`. The test sets up
/// creatures and artifacts on both sides alongside lands, casts `Ondu Inversion`, and verifies
/// that all creatures and artifacts are destroyed while all lands remain on the battlefield.
#[test]
fn ondu_inversion_destroys_all_nonland_permanents_on_both_sides() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(718, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[plains(), llanowar_elves(), quiet_artifact()])
        .hand(0, &[ondu_inversion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "p0 starts with an elf"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "p1 starts with an elf"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "p0 starts with an artifact"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "p1 starts with an artifact"
    );

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, ondu_inversion());

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "p0's creature was destroyed"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "p1's creature was destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "p0's artifact was destroyed"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "p1's artifact was destroyed"
    );

    assert!(
        on_battlefield(&engine, p0, plains()).is_some(),
        "p0's lands survive the nonland wipe"
    );
    assert!(
        on_battlefield(&engine, p1, plains()).is_some(),
        "p1's lands survive the nonland wipe"
    );
    assert!(
        in_graveyard(&engine, p0, ondu_inversion()).is_some(),
        "the sorcery resolved to the graveyard"
    );
}
