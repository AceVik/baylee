//! Kavu Chameleon — {3}{G}{G} — Creature — Kavu
//! Oracle: This spell can't be countered.
//! Oracle: {G}: This creature becomes the color of your choice until end of turn.

use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

#[test]
fn kavu_chameleon_cannot_be_countered_and_changes_its_own_color_until_eot() {
    let chameleon = card_index("82c583ab-8158-4074-a3ff-b7a05ecc8ab1");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .battlefield(1, &[island(), island()])
        .hand(0, &[chameleon])
        .hand(1, &[index::COUNTERSPELL])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, chameleon);
    let spell_id = engine.state().zones.list(ZoneLocation::Stack)[0];
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // Opponent casts Counterspell targeting Kavu Chameleon; Kavu Chameleon can't be countered.
    cast_from_hand(&mut engine, p1, index::COUNTERSPELL);
    aim_at(&mut engine, p1, spell_id);
    pass_until(&mut engine, stack_is_empty);

    let chameleon_id = on_battlefield(&engine, p0, chameleon).expect("Kavu Chameleon resolves");
    assert_eq!(pt(&engine, chameleon_id), (4, 4));
    assert_eq!(
        engine
            .state()
            .object(chameleon_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Green])
    );

    // Activate "{G}: This creature becomes the color of your choice until end of turn."
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, chameleon, 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(chameleon_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Blue])
    );

    // Reverts to green at end of turn.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine
            .state()
            .object(chameleon_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Green])
    );
}
