//! Defiling Tears — {2}{B} — Instant
//! Oracle: Until end of turn, target creature becomes black, gets +1/-1, and gains "{B}: Regenerate this creature."

use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

#[test]
fn defiling_tears_recolors_pumps_and_grants_black_regeneration_until_eot() {
    let tears = card_index("0decd8ea-a3d6-436b-81e9-bdd836537a1c");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), grizzly_bears()])
        .hand(0, &[tears, index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bears_id = on_battlefield(&engine, p0, grizzly_bears()).expect("Grizzly Bears on board");
    cast_from_hand(&mut engine, p0, tears);
    aim_at(&mut engine, p0, bears_id);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears_id), (3, 1));
    assert_eq!(
        engine
            .state()
            .object(bears_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Black])
    );

    crate::engine::card_tests::legends_kit::float(
        &mut engine,
        p0,
        &[(ManaColor::Black, 1), (ManaColor::Red, 1)],
    );

    // Activate the granted "{B}: Regenerate this creature" ability on Grizzly Bears.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bears_id,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Deal lethal damage with Lightning Bolt: Bears regenerates and stays on the battlefield tapped.
    cast_with_floating(&mut engine, p0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, p0, bears_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
    assert!(
        engine
            .state()
            .object(bears_id)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}
