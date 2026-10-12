//! Physical printing restrictions and choices exercised through real mana payments.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::card_metadata::{DeckChoices, RulePrint};
use baylee_core::color::{Color, ColorPair};
use baylee_core::ids::WatermarkId;

fn printing(e: &mut Engine<RegistryLookup>, object: ObjectId, mark: Option<u16>, is_un: bool) {
    let obj = e.state.object_mut(object).unwrap();
    obj.physical_rules = Some(std::sync::Arc::new(RulePrint {
        card: obj.card.unwrap().index,
        watermarks: vec![mark.map(WatermarkId::new)],
        is_un,
    }));
    e.refresh_offer();
}

#[test]
fn batch46_watermarket_spends_only_on_a_verified_watermarked_spell() {
    for marked in [false, true] {
        let mut e = prepared(&[index::SOL_RING], &[index::WATERMARKET], &[]);
        let spell = in_hand(&e, P0, index::SOL_RING).unwrap();
        printing(&mut e, spell, marked.then_some(0), false);
        reset_pool(&mut e);
        activate(&mut e, P0, index::WATERMARKET, 0);
        assert_eq!(e.state.players[0].mana_pool.total(), 2);
        assert_eq!(
            crate::casting::can_cast(e.state(), &RegistryLookup, P0, spell).is_ok(),
            marked
        );
        if marked {
            cast(&mut e, spell);
            assert_eq!(e.state.players[0].mana_pool.total(), 1);
        }
    }
}

#[test]
fn batch46_secret_base_freezes_the_producing_watermark_in_its_mana() {
    for target_mark in [None, Some(0), Some(1)] {
        let mut e = prepared(&[index::SOL_RING], &[index::SECRET_BASE], &[]);
        let base = on_battlefield(&e, P0, index::SECRET_BASE).unwrap();
        let spell = in_hand(&e, P0, index::SOL_RING).unwrap();
        printing(&mut e, base, Some(0), true);
        printing(&mut e, spell, target_mark, false);
        reset_pool(&mut e);
        activate(&mut e, P0, index::SECRET_BASE, 1);
        e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
        // A later source fact cannot relabel mana already produced.
        printing(&mut e, base, Some(1), true);
        assert_eq!(
            crate::casting::can_cast(e.state(), &RegistryLookup, P0, spell).is_ok(),
            target_mark == Some(0)
        );
        if target_mark == Some(0) {
            cast(&mut e, spell);
            assert_eq!(e.state.players[0].mana_pool.total(), 0);
        }
    }
}

#[test]
fn batch46_unknown_printing_metadata_is_not_a_verified_empty_printing() {
    let mut e = prepared(&[index::SOL_RING], &[index::WATERMARKET], &[]);
    let spell = in_hand(&e, P0, index::SOL_RING).unwrap();
    e.state.object_mut(spell).unwrap().physical_rules = None;
    reset_pool(&mut e);
    activate(&mut e, P0, index::WATERMARKET, 0);
    assert!(crate::casting::can_cast(e.state(), &RegistryLookup, P0, spell).is_err());
}

#[test]
fn batch46_underdome_pays_un_spell_and_un_activated_source_only() {
    for is_un in [false, true] {
        let mut e = prepared(
            &[index::SOL_RING],
            &[index::UNDERDOME, index::STAFF_OF_DOMINATION],
            &[],
        );
        let spell = in_hand(&e, P0, index::SOL_RING).unwrap();
        let staff = on_battlefield(&e, P0, index::STAFF_OF_DOMINATION).unwrap();
        printing(&mut e, spell, None, is_un);
        printing(&mut e, staff, None, is_un);
        reset_pool(&mut e);
        activate(&mut e, P0, index::UNDERDOME, 1);
        e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
        assert_eq!(
            crate::casting::can_cast(e.state(), &RegistryLookup, P0, spell).is_ok(),
            is_un
        );
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority");
        };
        assert_eq!(legal.abilities.contains(&(staff, 0)), is_un);
        if is_un {
            e.apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: staff,
                    ability_index: 0,
                },
            )
            .unwrap();
            assert_eq!(e.state.players[0].mana_pool.total(), 0);
        }
    }
}

#[test]
fn batch46_cryptic_spires_produces_each_of_the_ten_circled_pairs() {
    for (at, first) in Color::ALL.iter().enumerate() {
        for second in &Color::ALL[at + 1..] {
            let mut e = prepared(&[], &[index::CRYPTIC_SPIRES], &[]);
            let land = on_battlefield(&e, P0, index::CRYPTIC_SPIRES).unwrap();
            e.state.object_mut(land).unwrap().deck_choices = DeckChoices {
                circled_colors: Some(ColorPair::new(*first, *second)),
            };
            e.state.set_tapped(land, false);
            reset_pool(&mut e);
            activate(&mut e, P0, index::CRYPTIC_SPIRES, 0);
            let Pending::ChooseColor { options, .. } = e.pending() else {
                panic!("circled color choice");
            };
            let wanted = [
                ManaColor::from_color(*first),
                ManaColor::from_color(*second),
            ];
            assert_eq!(options, &wanted);
            e.apply(P0, PlayerAction::ChooseColor(wanted[1])).unwrap();
            assert_eq!(e.state.players[0].mana_pool.available(wanted[1]), 1);
        }
    }
}

#[test]
fn batch46_copy_retains_spires_circles_without_copying_physical_printing_facts() {
    let mut e = prepared(&[], &[index::CRYPTIC_SPIRES, index::FOREST], &[]);
    let spires = on_battlefield(&e, P0, index::CRYPTIC_SPIRES).unwrap();
    let copier = on_battlefield(&e, P0, index::FOREST).unwrap();
    e.state.object_mut(spires).unwrap().deck_choices = DeckChoices {
        circled_colors: Some(ColorPair::new(Color::Blue, Color::Green)),
    };
    printing(&mut e, spires, Some(0), true);
    printing(&mut e, copier, Some(1), false);
    assert!(crate::copiable_abilities::apply_copy(
        &mut e.state,
        crate::copiable_abilities::CopyApplication {
            source: copier,
            target: spires,
            mods: &[],
            copy_index: 0,
            until_eot: false,
            resolving: None,
            text: crate::text_changes::TextChangeMap::IDENTITY,
        }
    ));
    e.sync_static_effects();
    // Circles are copied text; a source's later choices cannot change a snapshot.
    e.state.object_mut(spires).unwrap().deck_choices = DeckChoices {
        circled_colors: Some(ColorPair::new(Color::White, Color::Black)),
    };
    let object = e.state.object(copier).unwrap();
    assert_eq!(object.watermark(), Some(WatermarkId::new(1)));
    assert!(!object.is_un_card());
    reset_pool(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: copier,
            ability_index: 0,
        },
    )
    .unwrap();
    let Pending::ChooseColor { options, .. } = e.pending() else {
        panic!("copied circle choice");
    };
    assert_eq!(options, &[ManaColor::Blue, ManaColor::Green]);
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Green), 1);
}

#[test]
fn batch46_two_spires_presets_keep_distinct_circles_through_hydration_and_activation() {
    use baylee_core::ids::PrintRef;
    use baylee_core::preset::{DeckEntry, Finish, PrintInfo};

    let mut preset = baylee_cards::decks::probe_preset(SEED, index::FOREST).unwrap();
    let def = baylee_cards::by_index(index::CRYPTIC_SPIRES).unwrap();
    let print = PrintRef::new(u16::try_from(preset.prints.len()).unwrap());
    preset.prints.push(PrintInfo {
        scryfall_id: uuid::Uuid::parse_str(def.scryfall_id).unwrap(),
        lang: "EN".into(),
        finish: Finish::Normal,
    });
    let pairs = [
        ColorPair::new(Color::White, Color::Blue),
        ColorPair::new(Color::Black, Color::Red),
    ];
    let entries: Vec<_> = pairs
        .iter()
        .map(|pair| DeckEntry {
            card: index::CRYPTIC_SPIRES,
            print,
            rule_print: None,
            choices: DeckChoices {
                circled_colors: Some(*pair),
            },
        })
        .collect();
    preset.seats[0].starting_battlefield.clear();
    preset.seats[0].starting_hand = Some(entries);
    preset.seats[1].starting_battlefield.clear();
    preset.seats[1].starting_hand = Some(vec![]);
    baylee_cards::print_rules::hydrate_reference(&mut preset).unwrap();
    assert!(
        preset.seats[0]
            .starting_hand
            .as_ref()
            .unwrap()
            .iter()
            .all(|entry| entry.rule_print.is_some())
    );
    let mut e = Engine::new(&preset, RegistryLookup).unwrap();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for (at, pair) in pairs.into_iter().enumerate() {
        if at > 0 {
            let turn = e.state.turn.number;
            pass_until(&mut e, |e| {
                e.state.turn.number > turn
                    && e.state.turn.active == P0
                    && e.state.turn.phase == crate::turn::Phase::FirstMain
                    && matches!(e.pending(), Pending::Priority { player: P0, .. })
            });
        }
        let card = e
            .state
            .zones
            .list(ZoneLocation::Hand(P0))
            .iter()
            .copied()
            .find(|id| {
                e.state.object(*id).is_some_and(|o| {
                    o.card.is_some_and(|c| c.index == index::CRYPTIC_SPIRES)
                        && o.deck_choices.circled_colors == Some(pair)
                })
            })
            .unwrap();
        e.apply(P0, PlayerAction::PlayLand { card }).unwrap();
        let land = card;
        assert_eq!(
            e.state.object(land).unwrap().deck_choices.circled_colors,
            Some(pair)
        );
        assert!(is_tapped(&e, land), "each played Spires enters tapped");
        e.state.set_tapped(land, false);
        reset_pool(&mut e);
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .unwrap();
        let Pending::ChooseColor { options, .. } = e.pending().clone() else {
            panic!("circled colors");
        };
        let wanted: Vec<_> = Color::ALL
            .into_iter()
            .filter(|color| pair.contains(*color))
            .map(ManaColor::from_color)
            .collect();
        assert_eq!(options, wanted);
        e.apply(P0, PlayerAction::ChooseColor(options[0])).unwrap();
        assert_eq!(e.state.players[0].mana_pool.available(options[0]), 1);
        assert!(stack_is_empty(&e));
    }
}
