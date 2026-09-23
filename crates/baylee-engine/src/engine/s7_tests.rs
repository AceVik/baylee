use super::*;
use crate::choice::CastModeKind;
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
fn force_of_will() -> CardIndex {
    card_index("956381ba-6d37-4a8a-846c-bad79222dbee")
}
fn counterspell() -> CardIndex {
    card_index("cc187110-1148-4090-bbb8-e205694a39f5")
}
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
fn mulldrifter() -> CardIndex {
    card_index("24d0f5e7-0d9e-4b76-900e-a7274e80312d")
}
fn cyclonic_rift() -> CardIndex {
    card_index("d75b9c82-1b49-4c3e-a1b5-aeef57d6644b")
}
fn toxic_deluge() -> CardIndex {
    card_index("afaef788-34d1-460b-b884-9d7ae6ddeb18")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

fn preset(
    seed: u64,
    hand0: Vec<CardIndex>,
    bf0: Vec<CardIndex>,
    hand1: Vec<CardIndex>,
    bf1: Vec<CardIndex>,
) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60)
        .map(|i| entry(if i % 2 == 0 { island() } else { forest() }))
        .collect();
    let mk = |hand: Vec<CardIndex>, bf: Vec<CardIndex>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: Some(hand.into_iter().map(entry).collect()),
        starting_battlefield: bf.into_iter().map(entry).collect(),
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk(hand0, bf0), mk(hand1, bf1)],
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

/// Whether mana made now can still be spent on a spell cast now.
///
/// A pool empties as the step ends (CR 500.5), so a driver loop that taps
/// every land the moment one is offered spends its whole board in the upkeep
/// and reaches the main phase with nothing — and then never casts anything at
/// all.
fn in_a_main_phase(engine: &Engine<RegistryLookup>) -> bool {
    matches!(
        engine.state().turn.phase,
        Phase::FirstMain | Phase::SecondMain
    )
}

fn pass_once(engine: &mut Engine<RegistryLookup>) {
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
}

#[test]
#[allow(clippy::too_many_lines)] // scenario script — step-by-step readability beats extraction
fn force_of_will_pitch_cast_without_mana() {
    // p0 casts a creature; p1 pitches Force of Will (life + exile blue).
    let mut engine = Engine::new(
        &preset(
            31,
            vec![ondu_cleric(), plains(), forest()],
            vec![],
            vec![force_of_will(), counterspell()],
            vec![],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let life_p1 = engine.state().players[1].life;

    // Walk to p0's main, play a land, tap, cast the cleric.
    let mut guard = 0;
    let cleric = loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if !legal.lands.is_empty() {
                    engine
                        .apply(
                            player,
                            PlayerAction::PlayLand {
                                card: legal.lands[0],
                            },
                        )
                        .unwrap();
                } else if !legal.mana_abilities.is_empty() && in_a_main_phase(&engine) {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                } else if let Some(&card) = legal.castable.iter().find(|c| {
                    engine
                        .state()
                        .object(**c)
                        .is_some_and(|o| o.card.is_some_and(|d| d.index == ondu_cleric()))
                }) {
                    engine
                        .apply(player, PlayerAction::CastSpell { card })
                        .unwrap();
                    break card;
                } else {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
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
        assert!(guard < 100, "cleric never cast");
    };

    // p1 pitches FoW: cast mode choice → alternative cost.
    pass_once(&mut engine); // p0 passes after casting
    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    let fow = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p1))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == force_of_will()))
        })
        .unwrap();
    assert!(
        legal.castable.contains(&fow),
        "FoW must be castable via pitch"
    );
    engine
        .apply(p1, PlayerAction::CastSpell { card: fow })
        .unwrap();

    // With an empty pool only the pitch alternative is payable, so the
    // wizard auto-selects it and asks for targets directly.

    // Targets: the cleric spell.
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

    // Pitch choice: exile a blue card from hand (Force of Will itself is
    // excluded; the Counterspell is the only blue card).
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected pitch choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    assert_eq!((min, max), (1, 1));
    assert_eq!(options.len(), 1, "only Counterspell is blue");
    let pitch_card = options[0];
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![pitch_card],
            },
        )
        .unwrap();

    // Cost paid: 1 life, card exiled, FoW on stack.
    assert_eq!(engine.state().players[1].life, life_p1 - 1);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&pitch_card)
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Stack)
            .contains(&fow)
    );
}

#[test]
fn mulldrifter_evoke_draws_then_sacrifices() {
    let mut engine = Engine::new(
        &preset(
            32,
            vec![mulldrifter(), island(), island(), island()],
            vec![],
            vec![],
            vec![],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);

    // Play 3 islands over 3 turns, evoke the drifter for {2}{U}.
    let mut cast_done = false;
    let mut guard = 0;
    while !cast_done {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if !legal.lands.is_empty() {
                    engine
                        .apply(
                            player,
                            PlayerAction::PlayLand {
                                card: legal.lands[0],
                            },
                        )
                        .unwrap();
                } else if !legal.mana_abilities.is_empty() && in_a_main_phase(&engine) {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                } else if let Some(&card) = legal.castable.iter().find(|c| {
                    engine
                        .state()
                        .object(**c)
                        .is_some_and(|o| o.card.is_some_and(|d| d.index == mulldrifter()))
                }) {
                    engine
                        .apply(player, PlayerAction::CastSpell { card })
                        .unwrap();
                    // Choose the evoke (alternative) mode if offered.
                    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
                        let alt = options
                            .iter()
                            .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
                            .expect("evoke option exists");
                        engine.apply(p0, PlayerAction::ChooseMode(alt)).unwrap();
                    }
                    cast_done = true;
                } else {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
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
        assert!(guard < 100, "drifter never cast");
    }

    // Resolve spell → ETB draw 2 (trigger) → resolve → evoke-sacrifice.
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        let drifter = engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .iter()
            .copied()
            .find(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == mulldrifter()))
            });
        if drifter.is_some() {
            break;
        }
        guard += 1;
        assert!(guard < 40, "drifter never sacrificed");
    }
    // Drew two cards from the ETB trigger (hand: drifter + 3 islands →
    // play 3 lands, cast drifter, +2 drawn = 2).
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 2);
}

#[test]
fn cyclonic_rift_overload_mode_bounces_everything() {
    let mut engine = Engine::new(
        &preset(
            33,
            vec![cyclonic_rift()],
            vec![island(); 7],
            vec![],
            vec![ondu_cleric()],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Walk to p0's main phase (mana expires at step end — tap and cast in
    // the same step).
    let mut guard = 0;
    while !matches!(engine.state().turn.phase, Phase::FirstMain) || engine.state().turn.active != p0
    {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
        guard += 1;
        assert!(guard < 20);
    }

    // p0 casts the rift with overload ({6}{U} from 6 islands).
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if !legal.mana_abilities.is_empty() {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                    continue;
                }
                let rift = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
                engine
                    .apply(player, PlayerAction::CastSpell { card: rift })
                    .unwrap();
                let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
                    panic!("expected modes, got {:?}", engine.pending())
                };
                let overload = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
                    .unwrap_or_else(|| {
                        panic!(
                            "overload mode missing; options: {:?}",
                            options.iter().map(|o| o.kind).collect::<Vec<_>>()
                        )
                    });
                engine
                    .apply(p0, PlayerAction::ChooseMode(overload))
                    .unwrap();
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 40);
    }
    pass_once(&mut engine);
    pass_once(&mut engine);
    // The opponent's cleric bounced to their hand (overload hits all
    // opposing nonlands); p0's islands stay (they're lands).
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == ondu_cleric()))
            })
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p1))
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == ondu_cleric()))
            })
    );
}

/// A spell whose only spell ability is `AbilityDef::ModalSpell` is never
/// offered a mode-less way to cast it.
///
/// Every effect such a card prints sits under a mode, so resolution finds no
/// `AbilityDef::Spell` and — with no mode chosen — no `mode_index` either:
/// the spell went hand → stack → graveyard and did nothing. A claim sweep
/// found it on Damn ("Destroy target creature", and nothing in the journal is
/// a destruction); the four cards in the pool that print `ModalSpell` all had
/// it. Nothing becomes uncastable, which is what the second half asserts — a
/// mode's cost defaults to the face's, so `Mode(0)` is the option `Normal`
/// was pretending to be.
#[test]
fn a_modal_spell_is_not_offered_a_mode_less_cast() {
    let mut engine = Engine::new(
        &preset(
            33,
            vec![cyclonic_rift()],
            vec![island(); 7],
            vec![],
            vec![ondu_cleric()],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);

    let mut guard = 0;
    while !matches!(engine.state().turn.phase, Phase::FirstMain) || engine.state().turn.active != p0
    {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
        guard += 1;
        assert!(guard < 20);
    }
    let sources = match engine.pending().clone() {
        Pending::Priority { legal, .. } => legal.mana_abilities,
        other => panic!("expected priority, got {other:?}"),
    };
    for source in sources {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let rift = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: rift })
        .unwrap();

    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected modes, got {:?}", engine.pending())
    };
    let kinds: Vec<_> = options.iter().map(|o| o.kind).collect();
    assert!(
        !kinds.iter().any(|k| matches!(k, CastModeKind::Normal)),
        "a mode-less cast was offered: {kinds:?}"
    );
    assert!(
        kinds.iter().any(|k| matches!(k, CastModeKind::Mode(0))),
        "the printed mode is gone too: {kinds:?}"
    );
    let normal = engine
        .state()
        .object(rift)
        .and_then(|o| o.card)
        .and_then(|c| baylee_cards::by_index(c.index))
        .expect("the rift is card-backed")
        .faces[0]
        .mana_cost;
    let plain = options
        .iter()
        .find(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .expect("the printed mode");
    assert_eq!(plain.cost, normal, "the mode is not the printed cost");
}

#[test]
fn toxic_deluge_pays_x_life_and_debuffs() {
    let mut engine = Engine::new(
        &preset(
            34,
            vec![toxic_deluge()],
            vec![swamp(), swamp(), swamp()],
            vec![],
            vec![ondu_cleric()],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let life_start = engine.state().players[0].life;

    // Walk to p0's main phase first (deluge is sorcery-speed).
    let mut guard = 0;
    while !matches!(engine.state().turn.phase, Phase::FirstMain) || engine.state().turn.active != p0
    {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
        guard += 1;
        assert!(guard < 20);
    }

    // Cast with X = 1 (3 swamps: {2}{B} + 1 life).
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if !legal.mana_abilities.is_empty() {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                    continue;
                }
                let deluge = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
                engine
                    .apply(player, PlayerAction::CastSpell { card: deluge })
                    .unwrap();
                let Pending::ChooseNumber { .. } = engine.pending().clone() else {
                    panic!("expected X choice, got {:?}", engine.pending())
                };
                engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 40);
    }
    assert_eq!(engine.state().players[0].life, life_start - 1);
    pass_once(&mut engine);
    pass_once(&mut engine);
    // The cleric (1/1) dies to -1/-1.
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == ondu_cleric()))
            })
    );
}

/// Regression: an out-of-range X must be rejected against the offered
/// min/max — an unchecked X of `4_000_000_000` used to wrap `as i32`
/// negative and *gain* the caster ~294M life on Toxic Deluge.
#[test]
fn toxic_deluge_rejects_x_outside_the_offered_range() {
    let mut engine = Engine::new(
        &preset(
            34,
            vec![toxic_deluge()],
            vec![swamp(), swamp(), swamp()],
            vec![],
            vec![ondu_cleric()],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let life_start = engine.state().players[0].life;

    // Walk to p0's main phase first (deluge is sorcery-speed).
    let mut guard = 0;
    while !matches!(engine.state().turn.phase, Phase::FirstMain) || engine.state().turn.active != p0
    {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
        guard += 1;
        assert!(guard < 20);
    }

    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if !legal.mana_abilities.is_empty() {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                    continue;
                }
                let deluge = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
                engine
                    .apply(player, PlayerAction::CastSpell { card: deluge })
                    .unwrap();
                let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
                    panic!("expected X choice, got {:?}", engine.pending())
                };
                assert!(
                    engine
                        .apply(p0, PlayerAction::ChooseNumber(max + 1))
                        .is_err(),
                    "above the offered range must be rejected"
                );
                assert!(
                    engine
                        .apply(p0, PlayerAction::ChooseNumber(4_000_000_000))
                        .is_err(),
                    "the historical wrap-around value must be rejected"
                );
                assert!(
                    engine.apply(p0, PlayerAction::ChooseNumber(min)).is_ok(),
                    "a value inside the range still works"
                );
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 40);
    }
    // The rejected answers changed nothing, and the accepted X = 0 costs
    // no life.
    assert_eq!(engine.state().players[0].life, life_start);
}

/// Walks a duel to p1's priority with a creature spell of p0's on the stack,
/// and answers with p1's Force of Will and whether the engine calls it
/// castable.
///
/// The scenario `force_of_will_pitch_cast_without_mana` builds, kept in one
/// place so the two tests below differ in a hand and a battlefield and in
/// nothing else.
fn fow_offered_at_p1s_priority(
    seed: u64,
    hand1: Vec<CardIndex>,
    bf1: Vec<CardIndex>,
) -> (Engine<RegistryLookup>, ObjectId, bool) {
    let mut engine = Engine::new(
        &preset(
            seed,
            vec![ondu_cleric(), plains(), forest()],
            vec![],
            hand1,
            bf1,
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);

    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if !legal.lands.is_empty() {
                    engine
                        .apply(
                            player,
                            PlayerAction::PlayLand {
                                card: legal.lands[0],
                            },
                        )
                        .unwrap();
                } else if !legal.mana_abilities.is_empty() && in_a_main_phase(&engine) {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                } else if let Some(&card) = legal.castable.iter().find(|c| {
                    engine
                        .state()
                        .object(**c)
                        .is_some_and(|o| o.card.is_some_and(|d| d.index == ondu_cleric()))
                }) {
                    engine
                        .apply(player, PlayerAction::CastSpell { card })
                        .unwrap();
                    break;
                } else {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
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
        assert!(guard < 100, "the cleric was never cast");
    }
    pass_once(&mut engine);

    let p1 = PlayerId::new(1);
    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    let fow = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p1))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == force_of_will()))
        })
        .expect("Force of Will in p1's hand");
    let castable = legal.castable.contains(&fow);
    (engine, fow, castable)
}

/// A pitch cost with nothing to pitch is not an offer.
///
/// Force of Will's alternative cost is `{0}`, so the legality probe — which
/// asked the alternative list about its *mana* and nothing else — put the
/// card in `legal.castable` whenever the printed `{3}{U}{U}` was out of
/// reach. Press it with no second blue card in hand and the wizard reached
/// `PitchChoice`, found no candidate and reversed the whole cast (CR 601.2h):
/// a card drawn as playable that answers a click by putting the game back
/// where it was. The same shape as the `AltCondition::CommanderControlled`
/// bug, one field over.
///
/// The deck under this preset is islands and forests, which are lands and so
/// have no colour at all (CR 202.2) — nothing p1 draws can match
/// `HasColor(Blue)` by accident.
#[test]
fn force_of_will_with_nothing_blue_to_pitch_is_not_offered() {
    let (engine, fow, castable) =
        fow_offered_at_p1s_priority(31, vec![force_of_will(), plains()], vec![]);
    assert!(
        !castable,
        "Force of Will was offered as castable with no blue card to exile"
    );

    // And the refusal is the engine's, not the offer list's alone: naming
    // the card anyway is an error rather than a reversed cast.
    let mut engine = engine;
    assert!(
        engine
            .apply(PlayerId::new(1), PlayerAction::CastSpell { card: fow })
            .is_err(),
        "casting it anyway must be refused"
    );

    // The positive control lives in `force_of_will_pitch_cast_without_mana`:
    // the same seat with a Counterspell in hand casts it for free.
}

/// The second half of the same offer, and the one the legality probe cannot
/// see: with the printed cost payable the card is castable either way, and
/// what must not appear is the *mode*.
///
/// `cast_options` runs `can_afford` over each alternative cost, which
/// accepted `ExileFromHand` without looking at the hand — so a Force of Will
/// held up on five islands offered "pay {3}{U}{U}" and "pay {0}, exile a blue
/// card" alike, and the second reversed the cast when taken.
#[test]
fn a_pitch_mode_with_nothing_to_pitch_is_not_a_cast_option() {
    let five_islands = vec![island(), island(), island(), island(), island()];
    let (mut engine, fow, _) =
        fow_offered_at_p1s_priority(32, vec![force_of_will(), plains()], five_islands);
    let p1 = PlayerId::new(1);

    // `legal.castable` reads the *pool*, so the islands have to be tapped
    // before the printed cost is payable at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p1, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&fow),
        "five islands pay the printed {{3}}{{U}}{{U}}"
    );
    engine
        .apply(p1, PlayerAction::CastSpell { card: fow })
        .expect("the printed cost is payable");

    // One option, so the wizard never asks which — it goes straight to the
    // spell it counters. A pitch mode here would be a second option and this
    // would be `ChooseCastMode`.
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "the pitch mode was offered with nothing to pitch: {:?}",
        engine.pending()
    );
}

/// The X of a pay-X-life cost is bounded by the caster, not by a constant.
///
/// CR 119.4: a payment of more than zero life is legal only while the life
/// total is at least the amount. The range above is the *offer's* range and
/// it was `max: 50` for every X the wizard has ever asked about — printed in
/// a mana cost, where the mana is validated when the wizard finishes, or paid
/// in life, where nothing validated anything. `finish_cast` subtracts what
/// comes back and does not look at the total, so Toxic Deluge for X = 25 at
/// twenty life was accepted, took the caster to -5, and lost them the game to
/// a state-based action on the way to resolving.
///
/// That it landed on this cost is not chance:
/// `FaceDef.mandatory_additional_costs` is the one list with no `can_afford`
/// in front of it, which
/// `cast_wizard::paid_as_a_mandatory_additional_cost` says out loud. The
/// bound belongs on the question rather than on the answer, because the
/// engine advances only through choices it enumerated — a client cannot name
/// a number that was never offered.
#[test]
fn toxic_deluge_asks_for_no_more_life_than_the_caster_has() {
    let mut engine = Engine::new(
        &preset(
            34,
            vec![toxic_deluge()],
            vec![swamp(), swamp(), swamp()],
            vec![],
            vec![ondu_cleric()],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let life_start = engine.state().players[0].life;
    assert!(
        life_start < 50,
        "the caster has to be poorer than the old constant for this to say anything"
    );

    let mut guard = 0;
    while !matches!(engine.state().turn.phase, Phase::FirstMain) || engine.state().turn.active != p0
    {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
        guard += 1;
        assert!(guard < 20);
    }

    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if !legal.mana_abilities.is_empty() {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                    continue;
                }
                let deluge = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
                engine
                    .apply(player, PlayerAction::CastSpell { card: deluge })
                    .unwrap();
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 40);
    }

    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected the X choice, got {:?}", engine.pending())
    };
    assert_eq!(min, 0, "paying no life is legal at any total (CR 119.4b)");
    assert_eq!(
        max,
        u32::try_from(life_start).expect("a positive starting life"),
        "the whole life total is payable, and not one point of it more"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::ChooseNumber(max + 1))
            .is_err(),
        "one life more than the caster has must be refused, not paid"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_start,
        "the refused answer paid nothing"
    );

    // And the whole total still goes through: a player may pay every point
    // they have, and then lose to the state-based action that follows.
    engine
        .apply(p0, PlayerAction::ChooseNumber(max))
        .expect("paying the whole life total is a legal payment");
    assert_eq!(engine.state().players[0].life, 0);
}

/// A spell with `{X}` in its *printed* cost is asked what X is.
///
/// Toxic Deluge above reaches the same question through its mandatory
/// pay-X-life cost, which is the other half of `needs_x` — so the printed
/// `{X}` half was covered by nothing at all, and it was wrong. The stage read
/// the cost through `wizard_cost`, which substitutes the X chosen *so far*,
/// and so far is zero: `{X}{U}{U}{U}` arrived as `{0}{U}{U}{U}`,
/// `has_variable()` was false, and the spell was cast for X = 0 with nobody
/// asked. Nothing failed and nothing logged; the question simply never
/// happened, which is why no test noticed for as long as there have been X
/// spells in the pool.
#[test]
fn a_printed_x_is_asked_for_and_is_the_number_that_resolves() {
    // Commander's Insight, {X}{U}{U}{U}: "target player draws X cards".
    let insight = card_index("54d7d7f8-22cd-4859-b203-924d248b422b");
    let island = card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373");
    let mut engine = Engine::new(
        &preset(
            35,
            vec![insight],
            vec![island, island, island, island, island],
            vec![],
            vec![],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);

    let mut guard = 0;
    while !matches!(engine.state().turn.phase, Phase::FirstMain) || engine.state().turn.active != p0
    {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
        guard += 1;
        assert!(guard < 20);
    }

    // Every island, so X has more than one legal value to be asked about.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities {
        engine
            .apply(player, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }

    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("five islands pay {2}{U}{U}{U}");

    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!(
            "a printed X must be asked about, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(min, 0);
    assert!(max >= 2, "X of two has to be inside the offered range");
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();

    let Pending::ChoosePlayer { .. } = engine.pending().clone() else {
        panic!("expected the target player, got {:?}", engine.pending())
    };
    engine.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();

    // And X = 2 is the number that resolves, not the zero it used to be.
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let mut guard = 0;
    while engine.state().zones.list(ZoneLocation::Hand(p0)).len() == before {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
        guard += 1;
        assert!(guard < 6, "the spell never resolved");
    }
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 2,
        "X = 2 draws two cards"
    );
}

/// Heliod's Intervention, `{X}{W}{W}`: "Choose one — Destroy X target
/// artifacts and/or enchantments; or Target player gains twice X life."
fn heliods_intervention() -> CardIndex {
    card_index("e7564d66-767c-4cd9-a5f0-0f2488a4a74b")
}

/// Casts Heliod's Intervention off five floating Plains in p0's own main
/// phase, answers the mode question with `mode`, and returns what is asked
/// next.
fn cast_heliod_in_mode(
    engine: &mut Engine<crate::engine::testkit::RegistryLookup>,
    mode: usize,
) -> Pending {
    use crate::engine::testkit::{tap_all_mana, walk_to_own_main};
    let p0 = PlayerId::new(0);
    assert!(walk_to_own_main(engine, p0), "p0 reaches its own main");
    tap_all_mana(engine, p0);
    let card = crate::engine::testkit::in_hand(engine, p0, heliods_intervention())
        .expect("the spell is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("five Plains are floating");
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "a modal spell asks for its mode, got {:?}",
            engine.pending()
        )
    };
    let slot = options
        .iter()
        .position(|o| o.kind == CastModeKind::Mode(mode))
        .expect("every mode is offered while its targets exist");
    engine
        .apply(p0, PlayerAction::ChooseMode(slot))
        .expect("the slot came from the question");
    engine.pending().clone()
}

/// The same question after a **mode** has been chosen.
///
/// CR 601.2b announces the mode and then, in the same step, the value of X:
/// picking how to cast a spell does not answer what X is. The wizard jumped
/// from the mode straight to the targets, so every spell with an `{X}` and
/// more than one way to be cast went on the stack for X = 0 without anybody
/// being asked — the test above walks the one-option path only, which is why
/// it never saw this. Heliod's Intervention's second mode, "target player
/// gains twice X life", gained nothing at all.
#[test]
fn a_printed_x_is_asked_for_after_a_mode_is_chosen() {
    use crate::engine::testkit::{
        Duel, keep_mulligans, pass_until, quiet_artifact, stack_is_empty,
    };
    let p0 = PlayerId::new(0);
    // Seat 1's artifact is there so both modes are on offer and the mode is
    // really asked (CR 700.2a): the question this test is about comes after it.
    let mut engine = Duel::new(35, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[heliods_intervention()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);

    let Pending::ChooseNumber { min, max, .. } = cast_heliod_in_mode(&mut engine, 1) else {
        panic!(
            "a printed X must be asked about after the mode too, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(min, 0, "X may always be nothing");
    assert!(
        max >= 3,
        "five Plains pay {{X}}{{W}}{{W}} for X = 3: max = {max}"
    );
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();

    let Pending::ChoosePlayer { .. } = engine.pending().clone() else {
        panic!("expected the target player, got {:?}", engine.pending())
    };
    engine.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();

    let before = engine.state().players[0].life;
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        before + 6,
        "twice X for X = 3 is six life, not the nothing X = 0 gains"
    );
}

/// "Destroy X target …" destroys **every** target the caster chose.
///
/// `Effect::Destroy` read its target through `spec_object`, which is the
/// first entry of the resolution's target list, so a spell that chose two
/// destroyed one and looked as if it had worked — the defect `spec_objects`
/// was written for, left behind on this one effect. The two targets are two
/// copies of one card on purpose: whichever of them is first, the other has
/// to go too, and the third permanent is the control that nothing else did.
#[test]
fn destroy_reads_every_chosen_target_not_the_first() {
    use crate::engine::testkit::{
        Duel, in_graveyard, keep_mulligans, on_battlefield, pass_until, quiet_artifact,
        stack_is_empty,
    };
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Exploration, the enchantment that is left alone.
    let exploration = card_index("0c2841bb-038c-4fbf-8360-bc0a1522b58d");
    let mut engine = Duel::new(35, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[heliods_intervention()])
        .battlefield(1, &[quiet_artifact(), quiet_artifact(), exploration])
        .start();
    keep_mulligans(&mut engine);

    let Pending::ChooseNumber { max, .. } = cast_heliod_in_mode(&mut engine, 0) else {
        panic!("expected X to be asked, got {:?}", engine.pending())
    };
    assert!(
        max >= 2,
        "five Plains pay {{X}}{{W}}{{W}} for X = 2: max = {max}"
    );
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();

    let rings: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == quiet_artifact())
        })
        .collect();
    assert_eq!(rings.len(), 2, "both of seat 1's artifacts are standing");
    let Pending::ChooseTargets {
        min, max, options, ..
    } = engine.pending().clone()
    else {
        panic!("X = 2 asks for two targets, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (2, 2), "exactly X targets (CR 601.2c)");
    assert!(
        rings.iter().all(|r| options.contains(r)),
        "both artifacts are on offer: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: rings,
                players: vec![],
            },
        )
        .expect("two targets, and X was announced as two");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "neither artifact is left: the second target was destroyed as well as the first"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "they went to their owner's graveyard (CR 701.8a)"
    );
    assert!(
        on_battlefield(&engine, p1, exploration).is_some(),
        "the permanent nobody targeted is untouched"
    );
}
