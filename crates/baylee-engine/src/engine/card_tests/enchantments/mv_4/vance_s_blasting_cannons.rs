//! `cards/enchantments/mv_4/vance_s_blasting_cannons.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

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

#[test]
fn cannons_exiles_at_upkeep_and_cast_permission_expires_and_never_plays_lands() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for (card, cast_now) in [
        (index::ORNITHOPTER, true),
        (index::ORNITHOPTER, false),
        (forest(), false),
        (index::ISHGARD_THE_HOLY_SEE, false),
    ] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[vance_s_blasting_cannons()])
            .hand(0, &[card])
            .start();
        keep_mulligans(&mut e);
        let top = hand_to_library_top(&mut e, p0, card);
        reach_main_phase(&mut e, p0);
        assert_eq!(e.state().object(top).unwrap().zone, Zone::Exile);
        if card == index::ORNITHOPTER && cast_now {
            e.apply(p0, PlayerAction::CastSpell { card: top }).unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().object(top).unwrap().zone, Zone::Battlefield);
        } else if card == index::ORNITHOPTER {
            reach_their_main_phase(&mut e, PlayerId::new(1));
            reach_their_main_phase(&mut e, p0);
            assert!(
                e.apply(p0, PlayerAction::CastSpell { card: top }).is_err(),
                "permission expired"
            );
        } else {
            assert!(
                !e.state()
                    .play_permissions
                    .iter()
                    .any(|permission| permission.card == top)
            );
            assert!(e.apply(p0, PlayerAction::PlayLand { card: top }).is_err());
        }
    }
}
