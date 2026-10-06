//! `cards/enchantments/mv_4/vance_s_blasting_cannons.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Vance's Blasting Cannons` // `Spitfire Bastion` (`Coverage::Partial`):
/// "At the beginning of your upkeep, exile the top card of your library. If it's a nonland card,
/// you may cast that card this turn. Whenever you cast your third spell in a turn, you may transform
/// `Vance's Blasting Cannons`. // `{{T}}`: Add `{{R}}`. `{{2}}{{R}}`, `{{T}}`: `Spitfire Bastion` deals
/// 3 damage to any target."
///
/// Under `Coverage::Partial`, the upkeep exile-and-cast trigger is omitted, while the third-spell
/// transform trigger and the back face's abilities are implemented. The test casts three spells
/// in one turn using `dark_ritual()`, intercepts the optional transform prompt on the third cast,
/// accepts the transformation into the legendary land `Spitfire Bastion` on face 1, and taps
/// `Spitfire Bastion` for red mana.
#[test]
fn vance_s_blasting_cannons_transforms_on_third_spell_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(111, swamp())
        .battlefield(0, &[vance_s_blasting_cannons(), swamp(), swamp(), swamp()])
        .hand(0, &[dark_ritual(), dark_ritual(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cannons = on_battlefield(&engine, p0, vance_s_blasting_cannons()).expect("the Cannons");
    let cannons_was = identity(&engine, cannons);

    // Spell 1
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);

    // Spell 2
    cast_with_floating(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);

    // Spell 3 triggers the third-spell MayDo transform trigger
    cast_with_floating(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let bastion = on_battlefield(&engine, p0, vance_s_blasting_cannons())
        .expect("Spitfire Bastion on battlefield");
    assert_eq!(
        engine.state().object(bastion).map(|o| o.face_index),
        Some(1),
        "Vance's Blasting Cannons transformed to face 1"
    );
    assert_eq!(
        identity(&engine, bastion),
        cannons_was,
        "the same object, turned over: a transform changes no zone (CR 712.18)"
    );

    let t = types(&engine, bastion);
    assert!(t.contains(TypeSet::LAND), "Spitfire Bastion is a land");
    assert!(
        !t.contains(TypeSet::ENCHANTMENT),
        "Spitfire Bastion is no longer an enchantment"
    );

    activate(&mut engine, p0, vance_s_blasting_cannons(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "Spitfire Bastion tapped for one red mana"
    );
}
