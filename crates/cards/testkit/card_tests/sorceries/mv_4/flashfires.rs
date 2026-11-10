//! `cards/sorceries/mv_4/flashfires.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flashfires: "Destroy all Plains."
#[test]
fn flashfires_destroys_only_plains() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                forest(),
                island(),
                swamp(),
            ],
        )
        .hand(0, &[flashfires()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mountains = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| mountains.contains(&id));
    cast_with_floating(&mut engine, p0, flashfires());
    pass_until(&mut engine, stack_is_empty);

    assert!(all_on_battlefield(&engine, p0, plains()).is_empty());
    assert!(
        !all_on_battlefield(&engine, p0, forest()).is_empty(),
        "not other basics"
    );
    assert!(!all_on_battlefield(&engine, p0, island()).is_empty());
    assert!(!all_on_battlefield(&engine, p0, swamp()).is_empty());
}

/// "Destroy all Plains" is every Plains on the table, not the caster's: the
/// opponent's Plains and their Savannah (a Forest Plains dual, CR 305.6) go to
/// their graveyard while their Forest and Island stay. The Plains we control
/// goes too.
#[test]
fn flashfires_destroys_the_opponents_plains_and_duals() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let savannah = card_index("703243f0-8cb3-420f-958f-5fd4bde30293");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), plains()],
        )
        .battlefield(1, &[plains(), savannah, forest(), island()])
        .hand(0, &[flashfires()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mountains = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| mountains.contains(&id));
    cast_with_floating(&mut engine, p0, flashfires());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, plains()).is_none(),
        "their Plains"
    );
    assert!(
        on_battlefield(&engine, p1, savannah).is_none(),
        "their Savannah has the Plains subtype"
    );
    assert!(in_graveyard(&engine, p1, plains()).is_some());
    assert!(in_graveyard(&engine, p1, savannah).is_some());
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "their Forest stays"
    );
    assert!(
        on_battlefield(&engine, p1, island()).is_some(),
        "their Island stays"
    );
    assert!(
        on_battlefield(&engine, p0, plains()).is_none(),
        "no \"you control\": ours goes too"
    );
}
