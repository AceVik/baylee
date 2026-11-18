use super::*;

#[test]
fn runic_palette_has_many_foils_and_native_metadata_rejects_forged_prints() {
    let mut content = runic().unwrap();
    let bytes = serde_json::to_vec(&content).unwrap().len();
    assert!(
        bytes < 240 * 1024,
        "{bytes} content bytes must leave room in the 256 KiB publication envelope"
    );
    let foils = content
        .catalog
        .cards
        .iter()
        .filter(|card| {
            card.run.unwrap().presentation.unwrap().finish == baylee_core::preset::Finish::Foil
        })
        .count();
    assert!(foils * 4 >= content.catalog.cards.len());
    for starter in &content.starters {
        let commander = starter.formation.as_ref().unwrap().commander;
        assert_eq!(
            content
                .catalog
                .card(commander)
                .unwrap()
                .run
                .unwrap()
                .presentation
                .unwrap()
                .finish,
            baylee_core::preset::Finish::Foil
        );
    }
    validate_catalog(&content.catalog).unwrap();
    content.catalog.cards[0]
        .run
        .as_mut()
        .unwrap()
        .presentation
        .as_mut()
        .unwrap()
        .printing = content.catalog.cards[1]
        .run
        .unwrap()
        .presentation
        .unwrap()
        .printing;
    assert!(validate_catalog(&content.catalog).is_err());
    let legacy = expeditions().unwrap();
    assert!(
        legacy
            .catalog
            .cards
            .iter()
            .all(|card| card.run.unwrap().presentation.is_none())
    );
    assert!(
        !serde_json::to_string(&legacy.catalog)
            .unwrap()
            .contains("presentation")
    );
}

#[test]
fn fifty_build_arounds_pass_native_admission_with_real_mana_bases() {
    let content = expeditions().unwrap();
    validate_catalog(&content.catalog).unwrap();
    assert_eq!(content.starters.len(), 50);
    assert!(content.catalog.cards.len() <= baylee_tower::cards::MAX_CATALOG);
    let mut with_nonbasics = 0;
    for starter in &content.starters {
        validate_seat(&seat(starter)).unwrap();
        let commander =
            baylee_cards::by_index(starter.formation.as_ref().unwrap().commander).unwrap();
        for color in commander.color_identity.iter() {
            let basic = baylee_cards::decks::basic_lands()[color as usize].unwrap();
            assert!(
                starter.deck.iter().any(|row| row.card == basic),
                "{} has no basic search target for {color:?}",
                commander.name()
            );
        }
        let mut lands = 0;
        let mut nonbasics = 0;
        for row in &starter.deck {
            let definition = baylee_cards::by_index(row.card).unwrap();
            if definition.faces[0]
                .types
                .contains(baylee_core::types::TypeSet::LAND)
            {
                lands += u16::from(row.copies);
                if !baylee_cards::formats::is_basic_land(row.card) {
                    nonbasics += 1;
                    assert_eq!(starter.copies(row.card), 1);
                }
            }
        }
        assert!((15..=18).contains(&lands), "land count {lands}");
        with_nonbasics += usize::from(nonbasics > 0);
    }
    assert!(
        with_nonbasics >= 40,
        "only {with_nonbasics} developed mana bases"
    );
    // A legal generic pile is not a tribal build-around. These independent
    // checks inspect the final native definitions, after builder refinement.
    for (leader, tribes) in [
        (
            "Rin and Seri, Inseparable",
            vec![
                baylee_core::generated::subtypes::creature::CAT,
                baylee_core::generated::subtypes::creature::DOG,
            ],
        ),
        (
            "Giada, Font of Hope",
            vec![baylee_core::generated::subtypes::creature::ANGEL],
        ),
        (
            "Dwynen, Gilt-Leaf Daen",
            vec![baylee_core::generated::subtypes::creature::ELF],
        ),
        (
            "Pashalik Mons",
            vec![baylee_core::generated::subtypes::creature::GOBLIN],
        ),
    ] {
        let leader_card = baylee_cards::decks::by_name(leader).unwrap();
        let starter = content
            .starters
            .iter()
            .find(|starter| starter.formation.as_ref().unwrap().commander == leader_card)
            .unwrap();
        let tribal = starter
            .deck
            .iter()
            .filter(|row| {
                let face = &baylee_cards::by_index(row.card).unwrap().faces[0];
                tribes.iter().any(|tribe| face.subtypes.contains(tribe))
            })
            .count();
        assert!(tribal >= 6, "{leader}: only {tribal} tribal spells");
    }
    // Standard Commander keeps its independent hundred-card size contract.
    let first = seat(&content.starters[0]);
    let main: Vec<_> = first.deck.iter().map(|row| row.card).collect();
    assert!(
        !baylee_cards::formats::check(
            baylee_cards::formats::Shape::Commander,
            &main,
            &[first.commanders[0].card]
        )
        .is_empty()
    );
}

#[test]
fn five_distinct_starters_use_only_tested_cards_and_actual_commander_identity() {
    let mut unavailable = Vec::new();
    for template in starters::ALL {
        for name in std::iter::once(&template.commander).chain(template.spells) {
            if named(name).is_err() {
                unavailable.push(*name);
            }
        }
        assert_eq!(template.spells.len(), 22);
        assert_eq!(
            template
                .lands
                .iter()
                .map(|(_, n)| u16::from(*n))
                .sum::<u16>(),
            17
        );
    }
    assert!(
        unavailable.is_empty(),
        "unavailable or untested cards: {unavailable:?}"
    );
    let content = standard().unwrap();
    validate_catalog(&content.catalog).unwrap();
    assert_eq!(content.starters.len(), 5);
    for collection in &content.starters {
        collection.validate(&content.catalog).unwrap();
        assert_eq!(collection.owned.len(), 23);
        assert!(collection.formation.as_ref().unwrap().sideboard.is_empty());
    }
}

fn seat(collection: &Collection) -> SeatSpec {
    use baylee_core::{card_metadata::DeckChoices, ids::PrintRef, preset::DeckEntry};
    let entry = |card| DeckEntry {
        card,
        print: PrintRef::new(0),
        rule_print: None,
        choices: DeckChoices::default(),
    };
    let forest = named("Forest").unwrap().card;
    let mut seat = baylee_cards::decks::probe_preset(37, forest)
        .unwrap()
        .seats
        .remove(0);
    seat.deck = collection
        .deck
        .iter()
        .flat_map(|row| (0..row.copies).map(move |_| entry(row.card)))
        .collect();
    seat.commanders = vec![entry(collection.formation.as_ref().unwrap().commander)];
    seat.sideboard.clear();
    seat
}

#[test]
fn native_admission_rechecks_colors_singleton_and_eligibility_from_the_registry() {
    let content = standard().unwrap();
    for starter in &content.starters {
        validate_seat(&seat(starter)).unwrap();
    }
    let valid = seat(&content.starters[0]);
    let forest = *valid
        .deck
        .iter()
        .find(|row| row.card == named("Forest").unwrap().card)
        .unwrap();
    let mut invalid = valid.clone();
    invalid.deck[0].card = named("Lightning Bolt").unwrap().card;
    assert!(validate_seat(&invalid).is_err());
    invalid = valid.clone();
    invalid.deck[0].card = invalid.commanders[0].card;
    assert!(validate_seat(&invalid).is_err());
    invalid = valid.clone();
    invalid.commanders[0] = forest;
    assert!(validate_seat(&invalid).is_err());
    invalid = valid.clone();
    invalid.sideboard = vec![forest; 5];
    assert!(validate_seat(&invalid).is_ok());
    invalid.sideboard.push(invalid.sideboard[0]);
    assert!(validate_seat(&invalid).is_err());
    invalid = valid.clone();
    invalid.sideboard.push(invalid.commanders[0]);
    assert!(validate_seat(&invalid).is_err());
    invalid = valid.clone();
    invalid.deck[0].rule_print = Some(baylee_core::ids::RulePrintRef::new(0));
    assert!(validate_seat(&invalid).is_err());
    invalid = valid;
    invalid.deck[0].choices.circled_colors = Some(baylee_core::color::ColorPair::new(
        baylee_core::color::Color::Blue,
        baylee_core::color::Color::Red,
    ));
    assert!(validate_seat(&invalid).is_err());
}

#[test]
fn forged_commander_facts_cannot_admit_an_unreviewed_catalog() {
    let content = standard().unwrap();
    let mut forged = content.catalog.clone();
    let leader = content.starters[0].formation.as_ref().unwrap().commander;
    let row = forged
        .cards
        .iter_mut()
        .find(|row| row.card == leader)
        .unwrap();
    row.run.as_mut().unwrap().identity = baylee_core::color::ColorSet::EMPTY;
    assert!(
        forged.validate().is_ok(),
        "the authority checks structure, not executable rules"
    );
    assert!(validate_catalog(&forged).is_err());
    forged = content.catalog;
    let row = forged
        .cards
        .iter_mut()
        .find(|row| row.card == leader)
        .unwrap();
    row.run.as_mut().unwrap().commander = false;
    assert!(validate_catalog(&forged).is_err());
}

#[test]
fn all_fifty_runic_starters_have_balanced_curves_synergies_and_fetch_targets() {
    use baylee_core::types::{SupertypeSet, TypeSet};
    let content = runic().unwrap();
    assert_eq!(content.starters.len(), 50);
    for starter in &content.starters {
        let formation = starter.formation.as_ref().unwrap();
        let leader = baylee_cards::by_index(formation.commander).unwrap();
        let mut lands = 0_u16;
        let mut creatures = 0_u16;
        let mut spells_is = 0_u16;
        let mut artifacts = 0_u16;
        let mut equipment_auras = 0_u16;
        let mut legends = 0_u16;
        let mut mv_sum = 0_u32;
        let mut nonlands = 0_u32;
        for row in &starter.deck {
            let def = baylee_cards::by_index(row.card).unwrap();
            let face = &def.faces[0];
            let n = u16::from(row.copies);
            if face.types.contains(TypeSet::LAND) {
                lands += n;
            } else {
                nonlands += u32::from(n);
                mv_sum += face.mana_cost.cmc() * u32::from(n);
            }
            if face.types.contains(TypeSet::CREATURE) {
                creatures += n;
            }
            if face
                .types
                .intersects(TypeSet::INSTANT.union(TypeSet::SORCERY))
            {
                spells_is += n;
            }
            if face.types.contains(TypeSet::ARTIFACT) {
                artifacts += n;
            }
            if face
                .subtypes
                .contains(&baylee_core::generated::subtypes::artifact::EQUIPMENT)
                || face
                    .subtypes
                    .contains(&baylee_core::generated::subtypes::enchantment::AURA)
            {
                equipment_auras += n;
            }
            if face.supertypes.contains(SupertypeSet::LEGENDARY)
                && !face.types.contains(TypeSet::LAND)
            {
                legends += n;
            }
        }
        let avg_mv = f64::from(mv_sum) / f64::from(nonlands.max(1));
        assert!(
            (15..=18).contains(&lands),
            "{} has {lands} lands",
            leader.name()
        );
        assert!(
            avg_mv <= 4.0,
            "{} curve too high: {avg_mv:.2}",
            leader.name()
        );
        for color in leader.color_identity.iter() {
            let basic = baylee_cards::decks::basic_lands()[color as usize].unwrap();
            let basic_copies: u8 = starter
                .deck
                .iter()
                .filter(|r| r.card == basic)
                .map(|r| r.copies)
                .sum();
            assert!(
                basic_copies >= 2,
                "{} has only {basic_copies} basic search targets for {color:?}",
                leader.name()
            );
        }
        match leader.name() {
            "Galazeth Prismari" => {
                assert!(
                    spells_is >= 6 && artifacts >= 6,
                    "Galazeth needs both spells ({spells_is}) and artifacts ({artifacts})"
                );
            }
            "Sram, Senior Edificer" => {
                assert!(
                    equipment_auras >= 8 && creatures >= 10,
                    "Sram needs equipment/auras ({equipment_auras}) and carriers ({creatures})"
                );
            }
            "Arvad the Cursed" => {
                assert!(legends >= 8, "Arvad needs legendary density ({legends})");
            }
            _ => {}
        }
    }
}

/// What the local stack and the held demo publish as the house's stand-ins is
/// a build the store accepts, at every stage and for every starter.
#[test]
fn every_development_house_is_a_legal_build_of_its_starter() {
    let content = runic().unwrap();
    for stage in 1..=baylee_tower::v2::MAX_STAGES {
        for starter in &content.starters {
            let build = development_house(starter, stage);
            build
                .validate_for(&content.catalog, baylee_tower::v2::GENERATOR)
                .unwrap();
            assert_eq!(build.deck, starter.deck);
            assert_eq!(build.formation, starter.formation);
            assert_eq!(build.active.is_empty(), stage < 2, "stage {stage}");
        }
    }
    let late = development_house(&content.starters[0], baylee_tower::v2::MAX_STAGES);
    assert_eq!(late.active.len(), 3);
}
