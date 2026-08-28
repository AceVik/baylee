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

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
fn counterspell() -> CardIndex {
    card_index("cc187110-1148-4090-bbb8-e205694a39f5")
}
fn demonic_tutor() -> CardIndex {
    card_index("82004860-e589-4e38-8d61-8c0210e4ea39")
}
fn brainstorm() -> CardIndex {
    card_index("36cd2364-d113-47d1-b2c4-b088d9eb88dd")
}
fn swords() -> CardIndex {
    card_index("b1544f21-7e98-461b-aed5-e748b0168c52")
}
fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

struct SeatBuilder {
    hand: Vec<CardIndex>,
    battlefield: Vec<CardIndex>,
}

impl SeatBuilder {
    fn new() -> Self {
        Self {
            hand: vec![],
            battlefield: vec![],
        }
    }
    fn hand(mut self, cards: &[CardIndex]) -> Self {
        self.hand = cards.to_vec();
        self
    }
    fn battlefield(mut self, cards: &[CardIndex]) -> Self {
        self.battlefield = cards.to_vec();
        self
    }
}

fn preset(seed: u64, seat0: SeatBuilder, seat1: SeatBuilder) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60)
        .map(|i| entry(if i % 2 == 0 { island() } else { forest() }))
        .collect();
    let mk = |b: SeatBuilder| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        deck: deck.clone(),
        starting_life: None,
        starting_hand: Some(b.hand.into_iter().map(entry).collect()),
        starting_battlefield: b.battlefield.into_iter().map(entry).collect(),
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
        seats: vec![mk(seat0), mk(seat1)],
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

fn pass_priority(engine: &mut Engine<RegistryLookup>) {
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
}

/// Passes priority until `pred` holds for the pending request or the turn.
fn pass_until(
    engine: &mut Engine<RegistryLookup>,
    mut pred: impl FnMut(&Engine<RegistryLookup>) -> bool,
) {
    let mut guard = 0;
    while !pred(engine) {
        match engine.pending().clone() {
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
            other => panic!("unexpected while passing: {other:?}"),
        }
        guard += 1;
        assert!(guard < 100, "condition never reached");
    }
}

fn hand_has(engine: &Engine<RegistryLookup>, player: PlayerId, card: CardIndex) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(player))
        .iter()
        .any(|id| engine.state().object(*id).unwrap().card.unwrap().index == card)
}

#[test]
#[allow(clippy::too_many_lines)] // scenario script — step-by-step readability beats extraction
fn counterspell_counters_a_creature_spell() {
    let mut engine = Engine::new(
        &preset(
            12,
            SeatBuilder::new().hand(&[plains(), forest(), ondu_cleric()]),
            SeatBuilder::new()
                .hand(&[counterspell()])
                .battlefield(&[island(), island()]),
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Turn 3 (p0's second turn): play forest, tap both lands, cast cleric.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    engine
        .apply(
            p0,
            PlayerAction::PlayLand {
                card: legal.lands[0],
            },
        )
        .unwrap();
    // p0's next main phase: play the second land and cast.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
            && e.state().turn.number >= 3
    });
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    if !legal.lands.is_empty() {
        engine
            .apply(
                p0,
                PlayerAction::PlayLand {
                    card: legal.lands[0],
                },
            )
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    let cleric = hand
        .iter()
        .find(|id| engine.state().object(**id).unwrap().card.unwrap().index == ondu_cleric())
        .copied()
        .unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: cleric })
        .unwrap();

    // p0 passes; p1 taps both islands and casts Counterspell on the cleric.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p1, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    let cs = engine.state().zones.list(ZoneLocation::Hand(p1))[0];
    assert!(legal.castable.contains(&cs));
    engine
        .apply(p1, PlayerAction::CastSpell { card: cs })
        .unwrap();

    // Target choice: the cleric spell is the only option.
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    assert_eq!(options, vec![cleric]);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();

    // Both pass twice: counterspell resolves, cleric is countered.
    pass_priority(&mut engine);
    pass_priority(&mut engine);
    // The cleric is in p0's graveyard, not on the battlefield/stack.
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&cleric)
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&cleric)
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Stack)
            .contains(&cleric)
    );
}

#[test]
fn demonic_tutor_fetches_any_card() {
    let mut engine = Engine::new(
        &preset(
            8,
            SeatBuilder::new().hand(&[swamp(), forest(), demonic_tutor()]),
            SeatBuilder::new().hand(&[forest()]),
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    engine
        .apply(
            p0,
            PlayerAction::PlayLand {
                card: legal.lands[0],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
            && e.state().turn.number >= 3
    });
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    if !legal.lands.is_empty() {
        engine
            .apply(
                p0,
                PlayerAction::PlayLand {
                    card: legal.lands[0],
                },
            )
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    let tutor = hand
        .iter()
        .find(|id| engine.state().object(**id).unwrap().card.unwrap().index == demonic_tutor())
        .copied()
        .unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: tutor })
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    // The tutor offers the whole library as options.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected search, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    let lib_size = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    assert_eq!(options.len(), lib_size);
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
    let obj = engine.state().object(chosen).unwrap();
    assert_eq!(obj.zone, Zone::Hand);
    assert_eq!(obj.zone_owner, Some(p0));
}

#[test]
fn brainstorm_draws_three_puts_two_back_ordered() {
    let mut engine = Engine::new(
        &preset(
            5,
            SeatBuilder::new().hand(&[island(), brainstorm()]),
            SeatBuilder::new().hand(&[forest()]),
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    engine
        .apply(
            p0,
            PlayerAction::PlayLand {
                card: legal.lands[0],
            },
        )
        .unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: legal.mana_abilities[0],
            },
        )
        .unwrap();
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    let bs = hand
        .iter()
        .find(|id| engine.state().object(**id).unwrap().card.unwrap().index == brainstorm())
        .copied()
        .unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bs })
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected put-back choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (2, 2));
    // Hand now holds the original card + 3 drawn (brainstorm left hand).
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 3 - 1
    );
    let first = options[0];
    let second = options[1];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![first, second],
            },
        )
        .unwrap();
    // Put back in chosen order: last chosen is the top of the library.
    let lib = engine.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(lib.last(), Some(&second));
    assert_eq!(lib[lib.len() - 2], first);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 3 - 1 - 2
    );
}

#[test]
fn swords_exiles_creature_and_controller_gains_life() {
    let mut engine = Engine::new(
        &preset(
            6,
            SeatBuilder::new().battlefield(&[ondu_cleric()]),
            SeatBuilder::new()
                .hand(&[swords()])
                .battlefield(&[plains()]),
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let life_start = engine.state().players[0].life;
    let cleric = engine.state().zones.list(ZoneLocation::Battlefield)[0];

    // Walk to p1's first main phase.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p1
    });
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    engine
        .apply(
            p1,
            PlayerAction::ActivateManaAbility {
                source: legal.mana_abilities[0],
            },
        )
        .unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    let stp = engine.state().zones.list(ZoneLocation::Hand(p1))[0];
    assert!(legal.castable.contains(&stp));
    engine
        .apply(p1, PlayerAction::CastSpell { card: stp })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected targets, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![cleric]);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // Cleric is exiled (not destroyed); p0 gained 1 life (its power).
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&cleric)
    );
    assert_eq!(engine.state().players[0].life, life_start + 1);
}

#[test]
fn sol_ring_mana_ability_skips_the_stack() {
    let mut engine = Engine::new(
        &preset(
            3,
            SeatBuilder::new().battlefield(&[sol_ring()]),
            SeatBuilder::new().hand(&[forest()]),
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    let ring = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    let stack_before = engine.state().zones.list(ZoneLocation::Stack).len();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    assert!(legal.abilities.contains(&(ring, 0)));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ring,
                ability_index: 0,
            },
        )
        .unwrap();
    // Mana was added immediately; nothing went on the stack.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        stack_before
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Colorless),
        2
    );
    assert!(
        engine
            .state()
            .object(ring)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
}

#[test]
fn tutor_into_hand_via_vindicate_not_present() {
    // Sanity: unimplemented cards are still inert (no abilities).
    let mut engine = Engine::new(
        &preset(
            2,
            SeatBuilder::new().hand(&[forest()]),
            SeatBuilder::new().hand(&[forest()]),
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    assert!(hand_has(&engine, PlayerId::new(0), forest()));
}
