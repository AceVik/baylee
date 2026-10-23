#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

struct CombatCards(Vec<&'static baylee_cards_dsl::CardDef>);

impl baylee_engine::state::CardLookup for CombatCards {
    fn card(
        &self,
        index: baylee_core::ids::CardIndex,
    ) -> Option<&'static baylee_cards_dsl::CardDef> {
        self.0
            .iter()
            .find(|c| c.index == index)
            .copied()
            .or_else(|| baylee_cards::by_index(index))
    }
}

fn combat_card(
    index: u32,
    power: i16,
    toughness: i16,
    keywords: baylee_cards_dsl::KeywordSet,
) -> &'static baylee_cards_dsl::CardDef {
    use baylee_cards_dsl::{CardDef, FaceDef};
    Box::leak(Box::new(CardDef {
        index: baylee_core::ids::CardIndex::new(index),
        faces: Box::leak(Box::new([FaceDef {
            name: "Combat fixture",
            mana_cost: baylee_core::mana::ManaCost::from_symbol_generic(2),
            types: baylee_core::types::TypeSet::CREATURE,
            power: Some(power),
            toughness: Some(toughness),
            ..FaceDef::DEFAULT
        }])),
        keywords,
        ..CardDef::DEFAULT
    }))
}

#[test]
fn combat_lifelink_block_keeps_the_player_alive_in_the_engine() {
    use baylee_cards_dsl::KeywordSet;
    let cards = vec![
        combat_card(90_000, 2, 2, KeywordSet::LIFELINK),
        combat_card(90_001, 3, 3, KeywordSet::EMPTY),
        combat_card(90_002, 8, 8, KeywordSet::FLYING),
    ];
    let engine = combat_after_expert_blocks(cards, 8);
    assert_eq!(
        engine.state().players[0].life,
        2,
        "eight flying damage is survivable only by gaining two life from the ground block"
    );
    assert!(!matches!(engine.pending(), Pending::GameOver(_)));
}

#[test]
fn combat_first_strike_must_be_blocked_before_lifelink_can_happen() {
    use baylee_cards_dsl::KeywordSet;
    let cards = vec![
        combat_card(90_000, 3, 2, KeywordSet::LIFELINK),
        combat_card(90_001, 3, 3, KeywordSet::FIRST_STRIKE),
        combat_card(90_002, 1, 20, KeywordSet::EMPTY),
    ];
    let engine = combat_after_expert_blocks(cards, 3);
    assert_eq!(engine.state().players[0].life, 2);
    assert!(!matches!(engine.pending(), Pending::GameOver(_)));
}

/// #123, the owner's game played through the engine: eight 2/2s at twenty
/// life against one 75/75 first striker that attacks every turn it can. Its
/// controller is at forty, so that no attack of ours ends the game before
/// the question is asked — at twenty SHARP wins on turn 3, which is a
/// different game.
///
/// Turn 2 it attacks and is chumped. Turn 3 is the house AI's, the 75/75 is
/// still tapped, and every attack rule says swing; turn 4 it untaps and
/// comes back. Four of five profiles used to send everything on turn 3 (or,
/// SHARP, on turn 1 already) and meet turn 4 with every creature tapped, so
/// the engine had no block to offer and the game ended there. The seat must
/// still be alive after turn 4, must have attacked on turn 3 — a table that
/// never attacks would pass this too — and must have been offered a block
/// against the 75/75 on turn 4.
#[test]
fn the_swing_back_finds_a_creature_home_to_block_it() {
    use baylee_cards_dsl::KeywordSet;
    use baylee_core::ids::{CardIndex, Defender, PrintRef};
    let cards = vec![
        combat_card(90_000, 2, 2, KeywordSet::EMPTY),
        combat_card(90_001, 75, 75, KeywordSet::FIRST_STRIKE),
    ];
    let object = |index| DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: CardIndex::new(index),
        print: PrintRef::new(0),
    };
    for (name, profile) in [
        ("NOVICE", AIProfile::NOVICE),
        ("CASUAL", AIProfile::CASUAL),
        ("STEADY", AIProfile::STEADY),
        ("SHARP", AIProfile::SHARP),
        ("EXPERT", AIProfile::EXPERT),
    ] {
        let mut preset = position(&[], &[]);
        preset.seats[0].starting_battlefield = vec![object(90_000); 8];
        preset.seats[1].starting_battlefield = vec![object(90_001)];
        preset.seats[1].starting_life = Some(40);
        let mut engine = Engine::new(&preset, CombatCards(cards.clone())).unwrap();
        let agent = HeuristicAgent::new(profile);
        let me = PlayerId::new(0);
        let mut attacked_on_turn_3 = 0;
        let mut offered_on_turn_4 = 0;
        for seq in 0..1_000 {
            let Some(seat) = pending_player(engine.pending()) else {
                break;
            };
            let view = asked_view(engine.state(), seat, seq, engine.pending());
            if view.turn > 4 {
                break;
            }
            let action = match engine.pending() {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                _ if seat == me => agent.act(&view, engine.pending()),
                Pending::Priority { .. } => PlayerAction::PassPriority,
                Pending::ChooseAttackers { attackers, .. } => PlayerAction::DeclareAttackers {
                    attackers: attackers
                        .iter()
                        .map(|id| (*id, Defender::Player(me)))
                        .collect(),
                },
                Pending::ChooseBlockers { .. } => {
                    PlayerAction::DeclareBlockers { blockers: vec![] }
                }
                other => panic!("{name}: unexpected question for their seat: {other:?}"),
            };
            match (engine.pending(), &action) {
                (Pending::ChooseAttackers { .. }, PlayerAction::DeclareAttackers { attackers })
                    if seat == me && view.turn == 3 =>
                {
                    attacked_on_turn_3 = attackers.len();
                }
                (Pending::ChooseBlockers { blockers, .. }, _) if view.turn == 4 => {
                    offered_on_turn_4 = blockers.len();
                }
                _ => {}
            }
            engine.apply(seat, action).expect("a legal answer");
        }
        assert!(
            engine.state().players[0].life > 0 && !matches!(engine.pending(), Pending::GameOver(_)),
            "{name} did not survive the 75/75's swing back"
        );
        assert!(
            attacked_on_turn_3 > 0,
            "{name} did not attack on turn 3, so surviving says nothing about \
             what it kept home"
        );
        assert!(
            offered_on_turn_4 > 0,
            "{name}: the engine offered no block against the 75/75 on turn 4"
        );
    }
}

/// #75 through the engine: their 3/3 attacks our planeswalker on turn 2 and
/// our one 2/2 may block it. On three loyalty the block is what keeps the
/// walker (CR 120.3c, CR 704.5i); on nine it survives the hit and the 2/2
/// stays out. The walker prints no ability, so nothing the seat does on turn
/// 1 moves its loyalty.
#[test]
fn a_creature_blocks_to_keep_a_walker_the_attack_would_kill() {
    use baylee_cards_dsl::KeywordSet;
    use baylee_core::ids::{CardIndex, Defender, PrintRef};
    use baylee_core::types::TypeSet;
    let walker = |index: u32, loyalty: u16| -> &'static baylee_cards_dsl::CardDef {
        use baylee_cards_dsl::{CardDef, FaceDef};
        Box::leak(Box::new(CardDef {
            index: CardIndex::new(index),
            faces: Box::leak(Box::new([FaceDef {
                name: "Walker fixture",
                mana_cost: baylee_core::mana::ManaCost::from_symbol_generic(4),
                types: TypeSet::PLANESWALKER,
                loyalty: Some(loyalty),
                ..FaceDef::DEFAULT
            }])),
            ..CardDef::DEFAULT
        }))
    };
    let cards = vec![
        combat_card(90_000, 2, 2, KeywordSet::EMPTY),
        combat_card(90_001, 3, 3, KeywordSet::EMPTY),
        walker(90_002, 3),
        walker(90_003, 9),
    ];
    let object = |index| DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: CardIndex::new(index),
        print: PrintRef::new(0),
    };
    for (walker_index, loyalty, blocks) in [(90_002, 3, true), (90_003, 9, false)] {
        for (name, profile) in [
            ("NOVICE", AIProfile::NOVICE),
            ("CASUAL", AIProfile::CASUAL),
            ("STEADY", AIProfile::STEADY),
            ("SHARP", AIProfile::SHARP),
            ("EXPERT", AIProfile::EXPERT),
        ] {
            let mut preset = position(&[], &[]);
            preset.seats[0].starting_battlefield = vec![object(90_000), object(walker_index)];
            preset.seats[1].starting_battlefield = vec![object(90_001)];
            let mut engine = Engine::new(&preset, CombatCards(cards.clone())).unwrap();
            let agent = HeuristicAgent::new(profile);
            let me = PlayerId::new(0);
            let mut blocked = None;
            for seq in 0..1_000 {
                let Some(seat) = pending_player(engine.pending()) else {
                    break;
                };
                let view = asked_view(engine.state(), seat, seq, engine.pending());
                if view.turn > 2 {
                    break;
                }
                let target = view
                    .battlefield_of(me)
                    .find(|o| o.types.contains(TypeSet::PLANESWALKER))
                    .map(|o| o.id);
                let action = match engine.pending() {
                    Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                    _ if seat == me => agent.act(&view, engine.pending()),
                    Pending::Priority { .. } => PlayerAction::PassPriority,
                    Pending::ChooseAttackers { attackers, .. } => PlayerAction::DeclareAttackers {
                        attackers: attackers
                            .iter()
                            .map(|id| (*id, Defender::Planeswalker(target.expect("a walker"))))
                            .collect(),
                    },
                    Pending::ChooseBlockers { .. } => {
                        PlayerAction::DeclareBlockers { blockers: vec![] }
                    }
                    other => panic!("{name}: unexpected question for their seat: {other:?}"),
                };
                if seat == me
                    && let (
                        Pending::ChooseBlockers { .. },
                        PlayerAction::DeclareBlockers { blockers },
                    ) = (engine.pending(), &action)
                {
                    blocked = Some(!blockers.is_empty());
                }
                engine.apply(seat, action).expect("a legal answer");
            }
            assert_eq!(
                blocked,
                Some(blocks),
                "{name}, walker on {loyalty}: blocked {blocked:?}"
            );
            let view = player_view(engine.state(), me, 0, None, &SeatContext::default(), &[]);
            assert!(
                view.battlefield_of(me)
                    .any(|o| o.types.contains(TypeSet::PLANESWALKER)),
                "{name}: the walker on {loyalty} did not survive"
            );
        }
    }
}

#[test]
fn selected_effect_context_routes_positive_and_negative_counters_in_the_engine() {
    use baylee_cards_dsl::{
        AbilityDef, Amount, CardDef, CounterKind, Effect, FaceDef, TargetReq, TargetSpec,
    };
    use baylee_core::ids::{CardIndex, PrintRef};
    for (kind, controller) in [(CounterKind::P1P1, 0), (CounterKind::M1M1, 1)] {
        let spell = Box::leak(Box::new(CardDef {
            index: CardIndex::new(90_003),
            faces: Box::leak(Box::new([FaceDef {
                name: "Counter direction fixture",
                mana_cost: baylee_core::mana::ManaCost::from_symbol(
                    baylee_core::mana::ManaSymbol::Generic(0),
                ),
                types: baylee_core::types::TypeSet::SORCERY,
                ..FaceDef::DEFAULT
            }])),
            abilities: Box::leak(Box::new([AbilityDef::Spell {
                effects: Box::leak(Box::new([Effect::AddCounter {
                    kind,
                    amount: Amount::Fixed(1),
                }])),
                targets: Some(TargetReq::one(TargetSpec::Object(
                    &baylee_cards_dsl::Filter::CREATURE,
                ))),
                second_targets: None,
                condition: None,
            }])),
            ..CardDef::DEFAULT
        }));
        let object = |index| DeckEntry {
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
            card: CardIndex::new(index),
            print: PrintRef::new(0),
        };
        let mut preset = position(&[], &[]);
        preset.seats[0].starting_hand = Some(vec![object(90_003)]);
        preset.seats[0].starting_battlefield = vec![object(90_000)];
        preset.seats[1].starting_battlefield = vec![object(90_001)];
        let lookup = CombatCards(vec![
            spell,
            combat_card(90_000, 2, 2, baylee_cards_dsl::KeywordSet::EMPTY),
            combat_card(90_001, 4, 4, baylee_cards_dsl::KeywordSet::EMPTY),
        ]);
        let mut engine = Engine::new(&preset, lookup).unwrap();
        let agent = HeuristicAgent::new(AIProfile::EXPERT);
        let mut targeted = false;
        let mut resolved = false;
        for seq in 0..80 {
            let pending = engine.pending();
            let seat = pending_player(pending).expect("fixture stays live");
            let view = asked_view(engine.state(), seat, seq, pending);
            if view
                .battlefield
                .iter()
                .any(|o| o.controller == PlayerId::new(controller) && !o.counters.is_empty())
            {
                resolved = true;
                break;
            }
            let action = match pending {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                Pending::Priority { legal, .. }
                    if !targeted && seat == PlayerId::new(0) && !legal.castable.is_empty() =>
                {
                    PlayerAction::CastSpell {
                        card: legal.castable[0],
                    }
                }
                Pending::ChooseTargets { .. } => {
                    let context = engine.decision_context();
                    assert_eq!(
                        context.effects,
                        spell
                            .abilities
                            .iter()
                            .find_map(|a| match a {
                                AbilityDef::Spell { effects, .. } => Some(*effects),
                                _ => None,
                            })
                            .unwrap()
                    );
                    targeted = true;
                    agent.act_with_context(&view, pending, &context)
                }
                Pending::Priority { .. } => PlayerAction::PassPriority,
                _ => agent.act_with_context(&view, pending, &engine.decision_context()),
            };
            engine
                .apply(seat, action)
                .expect("counter decision is legal");
        }
        assert!(
            targeted && resolved,
            "counter must reach the intended side: {kind:?}, targeted={targeted}, pending={:?}",
            engine.pending()
        );
    }
}

fn combat_after_expert_blocks(
    cards: Vec<&'static baylee_cards_dsl::CardDef>,
    life: i32,
) -> Engine<CombatCards> {
    use baylee_core::ids::{CardIndex, Defender, PrintRef};
    let mut preset = position(&[], &[]);
    let object = |index| DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: CardIndex::new(index),
        print: PrintRef::new(0),
    };
    preset.seats[0].starting_life = Some(life);
    preset.seats[0].starting_battlefield = vec![object(90_000)];
    preset.seats[1].starting_battlefield = vec![object(90_001), object(90_002)];
    let mut engine = Engine::new(&preset, CombatCards(cards)).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let mut blocked = false;
    for seq in 0..200 {
        let Some(seat) = pending_player(engine.pending()) else {
            break;
        };
        let view = asked_view(engine.state(), seat, seq, engine.pending());
        if blocked && view.step == baylee_view::Step::CombatEnd {
            break;
        }
        let action = match engine.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { attackers, .. } if seat == PlayerId::new(1) => {
                PlayerAction::DeclareAttackers {
                    attackers: attackers
                        .iter()
                        .map(|id| (*id, Defender::Player(PlayerId::new(0))))
                        .collect(),
                }
            }
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers { attackers: vec![] },
            Pending::ChooseBlockers { .. } if seat == PlayerId::new(0) => {
                blocked = true;
                agent.act(&view, engine.pending())
            }
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
            _ => panic!("unexpected question: {:?}", engine.pending()),
        };
        engine
            .apply(seat, action)
            .expect("combat decision is legal");
    }
    assert!(blocked, "the fixture must reach the blocking choice");
    engine
}
