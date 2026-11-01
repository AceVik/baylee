//! Blurred Mongoose — {1}{G} — Creature — Mongoose
//! Oracle: This spell can't be countered.
//! Oracle: Shroud (This creature can't be the target of spells or abilities.)

use super::*;
use baylee_core::generated::index;

#[test]
fn blurred_mongoose_cannot_be_countered_and_has_shroud() {
    let mongoose = card_index("588a5bac-dcad-410e-8878-0230500c2e3a");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[island(), island()])
        .hand(0, &[mongoose])
        .hand(1, &[index::COUNTERSPELL])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, mongoose);
    let mongoose_spell = on_stack(&engine, mongoose).expect("Blurred Mongoose on stack");

    // P0 passes priority; P1 casts Counterspell targeting Blurred Mongoose.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, index::COUNTERSPELL);
    aim_at(&mut engine, p1, mongoose_spell);

    // Both spells resolve: Counterspell fails to counter Blurred Mongoose, which enters the battlefield.
    pass_until(&mut engine, stack_is_empty);

    let mongoose_id = on_battlefield(&engine, p0, mongoose).expect("Blurred Mongoose enters");
    assert_eq!(pt(&engine, mongoose_id), (2, 1));
    assert!(keywords(&engine, mongoose_id).contains(KeywordSet::SHROUD));
}
