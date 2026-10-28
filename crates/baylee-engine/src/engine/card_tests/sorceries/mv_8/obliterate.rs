//! Obliterate — {6}{R}{R} — Sorcery
//! Oracle: This spell can't be countered.
//! Oracle: Destroy all artifacts, creatures, and lands. They can't be regenerated.

use super::*;
use baylee_core::generated::index;

#[test]
fn obliterate_cannot_be_countered_and_destroys_artifacts_creatures_and_lands() {
    let obliterate = card_index("47496941-8007-45a0-a981-76c683cbddc1");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                index::GRIZZLY_BEARS,
                index::SOL_RING,
                index::CRUSADE,
            ],
        )
        .battlefield(1, &[island(), island()])
        .hand(0, &[obliterate])
        .hand(1, &[index::COUNTERSPELL])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bear = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(bear)
        .unwrap()
        .regeneration_shields = 1;
    cast_from_hand(&mut engine, p0, obliterate);
    let spell_id = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .first()
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    crate::engine::card_tests::legends_kit::float(&mut engine, p1, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut engine, p1, index::COUNTERSPELL);
    aim_at(&mut engine, p1, spell_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, mountain()).is_none());
    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, p0, index::SOL_RING).is_none());
    assert!(on_battlefield(&engine, p1, island()).is_none());
    // Enchantments are untouched by Obliterate.
    assert!(on_battlefield(&engine, p0, index::CRUSADE).is_some());
}
