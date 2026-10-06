//! `cards/sorceries/mv_4/shatterstorm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shatterstorm — {2}{R}{R} — "Destroy all artifacts. They can't be
/// regenerated."
///
/// Every artifact on both sides of the table leaves, and the Living Wall's
/// standing shield is not applied (CR 701.19c) — the Wall is an artifact
/// *creature*, which is the type the word "artifacts" has to catch on its
/// own. The creatures and lands beside them are untouched witnesses that the
/// sweep read the type rather than the class of permanent.
#[test]
fn shatterstorm_destroys_every_artifact_through_a_shield_and_spares_the_rest() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                living_wall(),
                quiet_artifact(),
                llanowar_elves(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[quiet_artifact(), rib_cage_spider()])
        .hand(0, &[shatterstorm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p0, living_wall()).expect("the Wall is out");
    let lands = all_on_battlefield(&engine, p0, mountain());
    assert_eq!(lands.len(), 5, "five Mountains are seated");

    // The Wall buys its own shield first, and the sweep ignores it.
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: lands[0] })
        .unwrap();
    raise_a_shield(&mut engine, p0, wall, 0);

    cast_from_hand(&mut engine, p0, shatterstorm());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, living_wall()).is_none(),
        "the shielded artifact creature died"
    );
    assert!(in_graveyard(&engine, p0, living_wall()).is_some());
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "so did the artifact beside it"
    );
    assert!(in_graveyard(&engine, p0, quiet_artifact()).is_some());
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and the one across the table"
    );
    assert!(in_graveyard(&engine, p1, quiet_artifact()).is_some());

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "a creature that is no artifact stands"
    );
    assert!(
        on_battlefield(&engine, p1, rib_cage_spider()).is_some(),
        "on both sides of the table"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, mountain()).len(),
        5,
        "and so do the lands"
    );
}
