use super::*;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};

struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

fn card_index(oracle_id: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle_id)
        .expect("card exists")
        .index
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn hallowed_fountain() -> CardIndex {
    card_index("f1750962-a87c-49f6-b731-02ae971ac6ea")
}
fn glacial_fortress() -> CardIndex {
    card_index("027dd013-baa7-4111-b3c9-f4d1414e9c45")
}
fn indatha_triome() -> CardIndex {
    card_index("ec2b3779-55f7-4169-aa66-6312fb52721f")
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

fn preset_with_hand(seed: u64, hand0: Vec<CardIndex>, bf0: Vec<CardIndex>) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(forest())).collect();
    let mk = |hand: Vec<CardIndex>, bf: Vec<CardIndex>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        deck: deck.clone(),
        starting_life: None,
        starting_hand: Some(hand.into_iter().map(entry).collect()),
        starting_battlefield: bf.into_iter().map(entry).collect(),
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        dev_mode: false,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk(hand0, bf0), mk(vec![], vec![])],
    }
}

fn keep_mulligans(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            other => panic!("expected mulligan, got {other:?}"),
        }
    }
}

fn play_land_from_hand(engine: &mut Engine<RegistryLookup>, p: PlayerId) {
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p && !legal.lands.is_empty() => {
                engine
                    .apply(
                        player,
                        PlayerAction::PlayLand {
                            card: legal.lands[0],
                        },
                    )
                    .unwrap();
                return;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 30, "no playable land for {p:?}");
    }
}

#[test]
fn shockland_pays_life_or_enters_tapped() {
    // Pay the 2 life.
    let mut engine = Engine::new(
        &preset_with_hand(41, vec![hallowed_fountain()], vec![]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let life_before = engine.state().players[0].life;
    play_land_from_hand(&mut engine, p0);
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        panic!("expected shockland choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.state().players[0].life, life_before - 2);
    let fountain = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    assert!(
        !engine
            .state()
            .object(fountain)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );

    // Decline → enters tapped.
    let mut engine = Engine::new(
        &preset_with_hand(42, vec![hallowed_fountain()], vec![]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    play_land_from_hand(&mut engine, p0);
    let Pending::YesNo { .. } = engine.pending().clone() else {
        panic!("expected shockland choice")
    };
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    let fountain = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    assert!(
        engine
            .state()
            .object(fountain)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
}

#[test]
fn checkland_condition_is_evaluated() {
    // Without a Plains/Island: enters tapped.
    let mut engine = Engine::new(
        &preset_with_hand(43, vec![glacial_fortress()], vec![]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    play_land_from_hand(&mut engine, p0);
    let fortress = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    assert!(
        engine
            .state()
            .object(fortress)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );

    // With a Plains: enters untapped.
    let mut engine = Engine::new(
        &preset_with_hand(44, vec![glacial_fortress()], vec![plains()]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    play_land_from_hand(&mut engine, p0);
    let fortress = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == glacial_fortress()))
        })
        .unwrap();
    assert!(
        !engine
            .state()
            .object(fortress)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
}

#[test]
fn triome_cycling_from_hand_draws() {
    let mut engine = Engine::new(
        &preset_with_hand(45, vec![indatha_triome(), forest(), forest()], vec![]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    // Play 2 forests, tap, cycle the triome from hand.
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                // Play FORESTS only — the triome must stay in hand to cycle.
                if let Some(&land) = legal.lands.iter().find(|id| {
                    engine
                        .state()
                        .object(**id)
                        .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
                }) {
                    engine
                        .apply(player, PlayerAction::PlayLand { card: land })
                        .unwrap();
                } else if !legal.mana_abilities.is_empty() {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                } else if let Some(&(card, idx)) = legal.abilities.iter().find(|(id, _)| {
                    engine
                        .state()
                        .object(*id)
                        .is_some_and(|o| o.card.is_some_and(|c| c.index == indatha_triome()))
                }) {
                    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
                    engine
                        .apply(
                            player,
                            PlayerAction::ActivateAbility {
                                source: card,
                                ability_index: idx,
                            },
                        )
                        .unwrap();
                    // Triome is in the graveyard (DiscardSelf), ability on stack.
                    assert!(
                        engine
                            .state()
                            .zones
                            .list(ZoneLocation::Graveyard(p0))
                            .contains(&card)
                    );
                    // Resolve the draw.
                    let Pending::Priority { player, .. } = engine.pending().clone() else {
                        panic!()
                    };
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                    let Pending::Priority { player, .. } = engine.pending().clone() else {
                        panic!()
                    };
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                    assert_eq!(
                        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
                        hand_before - 1 + 1, // cycled one, drew one
                        "cycling should net one draw"
                    );
                    return;
                } else {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 60, "triome never cycled");
    }
}
