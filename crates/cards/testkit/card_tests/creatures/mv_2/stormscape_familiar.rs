//! `cards/creatures/mv_2/stormscape_familiar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Stormscape Familiar — {1}{U} Creature — Bird 1/1
/// "Flying
/// White spells and black spells you cast cost {1} less to cast."
#[test]
fn stormscape_familiar_reduces_generic_cost_of_white_and_black_spells_you_cast() {
    let familiar = card_index("a8f5a11f-3291-4ee9-a96d-4fa38af14932");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                familiar,
                plains(),
                plains(),
                plains(),
                plains(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[index::SERRA_ANGEL, index::BOG_WRAITH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let familiar_id = on_battlefield(&engine, p0, familiar).unwrap();
    assert!(keywords(&engine, familiar_id).contains(KeywordSet::FLYING));

    // Cast Serra Angel ({3}{W}{W}) for {2}{W}{W} and Bog Wraith ({3}{B}) for {2}{B}.
    cast_from_hand(&mut engine, p0, index::SERRA_ANGEL);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, index::SERRA_ANGEL).is_some());

    cast_from_hand(&mut engine, p0, index::BOG_WRAITH);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, index::BOG_WRAITH).is_some());
}
