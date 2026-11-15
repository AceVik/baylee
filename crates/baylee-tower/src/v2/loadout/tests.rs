use super::*;
use crate::v2::catalog::{Family, Rarity};

fn preset() -> GamePreset {
    let resources = [
        resources(
            &[Definition {
                family: Family::Heartwood,
                rarity: Rarity::Rare,
            }],
            [1, 1],
            0,
        )
        .unwrap(),
        resources(
            &[Definition {
                family: Family::Satchel,
                rarity: Rarity::Uncommon,
            }],
            [0, 0],
            1,
        )
        .unwrap(),
    ];
    GamePreset {
        format: FormatId::Custom,
        seed: 19,
        house_rules: rules(&resources, 17).unwrap(),
        modifiers: Vec::new(),
        prints: vec![baylee_core::preset::PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "en".into(),
            finish: baylee_core::preset::Finish::Normal,
        }],
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        seats: resources
            .iter()
            .enumerate()
            .map(|(index, resource)| baylee_core::preset::SeatSpec {
                controller: if index == 0 {
                    SeatController::Open
                } else {
                    SeatController::Ai(generation::CURRENT.expert())
                },
                capabilities: SeatCapabilities::default(),
                deck: vec![baylee_core::preset::DeckEntry {
                    card: baylee_core::ids::CardIndex::new(0),
                    print: baylee_core::ids::PrintRef::new(0),
                    rule_print: None,
                    choices: baylee_core::card_metadata::DeckChoices::default(),
                }],
                sideboard: Vec::new(),
                commanders: Vec::new(),
                starting_life: Some(resource.life),
                starting_hand: None,
                starting_battlefield: Vec::new(),
                emblems: Vec::new(),
                team: None,
            })
            .collect(),
    }
}

#[test]
fn exact_native_resources_and_policy_are_required() {
    let good = preset();
    assert!(validate(&good).is_ok());
    assert_eq!(good.seats[0].starting_life, Some(31));
    assert_eq!(good.house_rules.opening[1].cards, 8);
    let edits: &[fn(&mut GamePreset)] = &[
        |p| p.house_rules.rule_pack = None,
        |p| p.house_rules.rule_pack.as_mut().unwrap().revision += 1,
        |p| {
            p.house_rules.rule_pack.as_mut().unwrap().seats[0]
                .definitions
                .push(u16::MAX);
        },
        |p| {
            p.house_rules.rule_pack.as_mut().unwrap().seats.remove(0);
        },
        |p| p.seats[0].starting_life = Some(32),
        |p| p.house_rules.opening[0].cards += 1,
        |p| p.house_rules.opening[0].free_mulligans += 1,
        |p| p.house_rules.supplies[0].seedling = true,
        |p| p.house_rules.supplies[0].healing += 1,
        |p| p.house_rules.loop_policy = LoopPolicy::RunOnceThenBreak,
        |p| p.house_rules.challenge_policy_seed = None,
        |p| p.house_rules.takebacks = true,
        |p| p.house_rules.reconnect_window_secs = 60,
        |p| p.seats[0].capabilities.dev_commands = true,
        |p| p.seats[1].capabilities.see_hidden = true,
        |p| p.seats[0].starting_hand = Some(Vec::new()),
        |p| p.seats[0].emblems.push("extra".into()),
        |p| p.seats[0].team = Some(0),
        |p| p.seats[1].controller = SeatController::Open,
    ];
    for (index, edit) in edits.iter().enumerate() {
        let mut bad = good.clone();
        edit(&mut bad);
        assert!(validate(&bad).is_err(), "forgery {index} admitted");
    }
}
