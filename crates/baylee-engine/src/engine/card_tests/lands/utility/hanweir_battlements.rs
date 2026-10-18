//! `cards/lands/utility/hanweir_battlements.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hanweir Battlements prints `{{T}}: Add {{C}}.`, `{{R}}, {{T}}: Target creature gains haste until end of turn.`, and `{{3}}{{R}}{{R}}, {{T}}: If you both own and control this land and a creature named Hanweir Garrison, exile them, then meld them into Hanweir, the Writhing Township.`
///
/// With `{{3}}{{R}}{{R}}` floating from five `mountain()` lands, a creature present (`llanowar_elves()`), and Hanweir Battlements untapped, all three abilities are offered in `legal.abilities`.
/// Activating ability 1 grants haste to the target creature until end of turn and taps the land.
#[test]
fn hanweir_battlements_grants_haste_and_offers_meld_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                hanweir_battlements(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let battlements = on_battlefield(&engine, p0, hanweir_battlements())
        .expect("hanweir battlements on battlefield");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf on battlefield");

    assert!(
        !keywords_of(&engine, elf).contains(KeywordSet::HASTE),
        "creature does not have haste initially"
    );

    // Float {{5}} red mana from the five Mountains while keeping Hanweir Battlements untapped.
    tap_mana_except(&mut engine, p0, battlements);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        5
    );
    assert!(!is_tapped(&engine, battlements));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(battlements, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(battlements, 1)),
        "ability 1 ({{R}}, {{T}}: Target creature gains haste) is offered"
    );
    assert!(
        legal.abilities.contains(&(battlements, 2)),
        "ability 2 (meld) is offered when {{3}}{{R}}{{R}} is floating"
    );

    activate(&mut engine, p0, hanweir_battlements(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords_of(&engine, elf).contains(KeywordSet::HASTE),
        "creature gained haste"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        4
    );
    assert!(is_tapped(&engine, battlements));
}

#[test]
fn hanweir_battlements_exiles_a_named_impostor_but_cannot_meld_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                hanweir_battlements(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let battlements = on_battlefield(&engine, p0, hanweir_battlements())
        .expect("hanweir battlements on battlefield");
    let garrison = on_battlefield(&engine, p0, llanowar_elves()).expect("partner on battlefield");

    let garrison_name = engine.state.names.intern("Hanweir Garrison");
    engine.state.object_mut(garrison).unwrap().base_mut().name = garrison_name;
    engine.state.object_mut(garrison).unwrap().cache.clear();
    engine.state.invalidate_projections();

    tap_mana_except(&mut engine, p0, battlements);
    activate(&mut engine, p0, hanweir_battlements(), 2);
    pass_until(&mut engine, stack_is_empty);

    for card in [battlements, garrison] {
        assert_eq!(
            engine.state().object(card).unwrap().zone,
            Zone::Exile,
            "matching names allow exile, but only the physical meld pair can return"
        );
    }
}

#[test]
fn real_hanweir_pair_melds_with_combined_mana_value_and_splits_on_bounce() {
    let p0 = PlayerId::new(0);
    let garrison_card = card_index("7cb29569-48e1-4782-9906-fad155ebfafe");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                hanweir_battlements(),
                garrison_card,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, hanweir_battlements()).unwrap();
    let creature = on_battlefield(&engine, p0, garrison_card).unwrap();
    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, hanweir_battlements(), 2);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, land), (7, 4));
    assert!(keywords(&engine, land).contains(KeywordSet::HASTE));
    assert_eq!(
        engine
            .state()
            .object(land)
            .unwrap()
            .characteristics()
            .mana_value(),
        3
    );
    assert!(engine.state().object(land).unwrap().is_card());
    assert!(
        engine
            .state()
            .object(land)
            .unwrap()
            .printed_face()
            .is_none()
    );
    assert_eq!(
        engine.state().merged_components(land).as_slice(),
        &[land, creature]
    );
    let _ = engine.state.move_object(
        land,
        ZoneLocation::Hand(p0),
        crate::zone::ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    for id in [land, creature] {
        let obj = engine.state().object(id).unwrap();
        assert_eq!(obj.zone, Zone::Hand);
        assert!(obj.token.is_none());
    }
    assert!(
        engine
            .state()
            .object(land)
            .unwrap()
            .base
            .types
            .contains(TypeSet::LAND)
    );
    assert_eq!(engine.state().object(creature).unwrap().base.power, Some(2));
}

#[test]
fn hanweir_garrison_attack_creates_two_tapped_attacking_red_humans() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let card = card_index("7cb29569-48e1-4782-9906-fad155ebfafe");
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &[card]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let garrison = on_battlefield(&engine, p0, card).unwrap();
    attack_and_collect_blocks(&mut engine, garrison, p1);
    let tokens: Vec<_> = engine
        .state()
        .battlefield_seen()
        .filter(|id| *id != garrison)
        .filter_map(|id| engine.state().object(id))
        .filter(|o| !o.is_card())
        .collect();
    assert_eq!(tokens.len(), 2);
    for token in tokens {
        assert_eq!((token.base.power, token.base.toughness), (Some(1), Some(1)));
        assert!(token.status.contains(Status::TAPPED));
        assert!(engine.state().combat.is_attacking(token.id));
        assert!(
            token
                .base
                .subtypes
                .contains(baylee_core::generated::subtypes::creature::HUMAN)
        );
    }
}

#[test]
fn meld_chooses_one_of_multiple_real_garrisons() {
    let p0 = PlayerId::new(0);
    let garrison = card_index("7cb29569-48e1-4782-9906-fad155ebfafe");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                hanweir_battlements(),
                garrison,
                garrison,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, hanweir_battlements()).unwrap();
    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, hanweir_battlements(), 2);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max, options.len()), (1, 1, 2));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[1]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().merged_components(land).as_slice(),
        &[land, options[1]]
    );
    assert_eq!(
        engine.state().object(options[0]).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(pt(&engine, options[0]), (2, 3));
    assert_eq!(pt(&engine, land), (7, 4));
}
