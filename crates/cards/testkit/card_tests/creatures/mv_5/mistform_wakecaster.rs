//! `cards/creatures/mv_5/mistform_wakecaster.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn mistform_wakecaster() -> CardIndex {
    card_index("7108f2a2-1541-4bbf-aeb8-2a892a6dc337")
}

/// Mistform Wakecaster has flying, `{1}` to change its own creature type
/// until end of turn, and `{2}{U}{U}, {T}` to change every creature you
/// control (leaving opponents' creatures untouched) until end of turn.
#[test]
fn mistform_wakecaster_changes_self_or_all_controlled_creatures_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                mistform_wakecaster(),
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[festering_goblin(), serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let caster = on_battlefield(&engine, p0, mistform_wakecaster()).expect("Wakecaster seated");
    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let goblin0 = on_battlefield(&engine, p0, festering_goblin()).expect("P0 Goblin");
    let goblin1 = on_battlefield(&engine, p1, festering_goblin()).expect("P1 Goblin");
    let angel1 = on_battlefield(&engine, p1, serra_angel()).expect("P1 Angel");
    assert!(keywords(&engine, caster).contains(KeywordSet::FLYING));

    // Ability 0 ({1}): change only Wakecaster's own subtype until EOT.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mistform_wakecaster(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::WIZARD))
        .expect("choose Wizard for self");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        engine
            .state()
            .object(caster)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::WIZARD)
    );
    assert!(
        engine
            .state()
            .object(elf0)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::ELF),
        "ability 0 leaves other creatures unchanged"
    );

    // Ability 1 ({2}{U}{U}, {T}): change every creature P0 controls, not P1's.
    activate(&mut engine, p0, mistform_wakecaster(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::DRAGON))
        .expect("choose Dragon for team");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(is_tapped(&engine, caster));
    for id in [caster, elf0, goblin0] {
        let subs = &engine
            .state()
            .object(id)
            .unwrap()
            .characteristics()
            .subtypes;
        assert!(subs.contains(subtypes::creature::DRAGON));
        assert!(!subs.contains(subtypes::creature::ELF));
        assert!(!subs.contains(subtypes::creature::GOBLIN));
    }
    assert!(
        engine
            .state()
            .object(goblin1)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::GOBLIN),
        "opponent creature keeps its type"
    );
    assert!(
        engine
            .state()
            .object(angel1)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::ANGEL)
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(
        engine
            .state()
            .object(caster)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::ILLUSION)
    );
    assert!(
        engine
            .state()
            .object(elf0)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::ELF)
    );
}
