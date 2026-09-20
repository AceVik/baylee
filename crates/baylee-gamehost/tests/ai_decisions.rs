//! Decisions tested across the real engine/view boundary.

use baylee_ai::{AIProfile, HeuristicAgent, pending_player};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{DeckEntry, GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_gamehost::{PlayerView, RegistryLookup, SeatContext, SeatKind, Session, player_view};

/// The view a seat gets at a decision point, as every fixture in this file
/// wants it: the seat being asked is the seat the table is waiting for, and
/// none of these tables ever opens a payment window.
fn asked_view(
    state: &baylee_engine::state::GameState,
    seat: PlayerId,
    seq: u64,
    pending: &Pending,
) -> PlayerView {
    player_view(
        state,
        seat,
        seq,
        Some(pending),
        &SeatContext {
            awaiting: Some(seat),
            ..Default::default()
        },
    )
}

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::decks::by_name(name).expect("registered fixture"),
        print: baylee_core::ids::PrintRef::new(0),
    }
}

fn position(hand: &[&str], board: &[&str]) -> GamePreset {
    let mut preset = baylee_cards::decks::probe_preset(41, entry("Forest").card).unwrap();
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(hand.iter().map(|name| entry(name)).collect());
    preset.seats[0].starting_battlefield = board.iter().map(|name| entry(name)).collect();
    preset
}

/// #87. The chair a `Session` seats is not keyed to the shuffle.
///
/// `Session::new` handed every AI seat `preset.seed` — the same stream that
/// dealt the hands and shuffled the libraries — and `docs/house-ai.md` wrote
/// it down as the rule. For a heuristic that only breaks ties that is
/// merely untidy; for anything that *samples* it is a leak no seat boundary
/// catches, because nothing crosses one: a sampler drawn from the stream
/// that produced the hidden state is correlated with the answer it is
/// supposed to be guessing at.
///
/// So the question is asked of chairs from games that differ in **nothing but
/// the seed**, over a fixture whose hand and battlefield are written into the
/// preset and therefore do not move with it. Every answer must match. The tie
/// is two identical Brainstorms, which is the shape the noise exists to
/// break — without one the assertion would be satisfied by an agent that has
/// no randomness to leak, which is what the test below holds separately.
#[test]
fn a_session_agent_does_not_inherit_the_seed_that_dealt_the_hands() {
    let base = position(&["Brainstorm", "Brainstorm"], &["Island", "Island"]);
    let seated = |seed: u64| {
        let mut preset = base.clone();
        preset.seed = seed;
        preset.seats[0].controller = SeatController::Ai(AIProfile::EXPERT);
        preset
    };
    let chair = |preset: &GamePreset| match Session::new(preset)
        .expect("the fixture builds a session")
        .seat_kind(PlayerId::new(0))
        .expect("seat zero is at the table")
    {
        SeatKind::Ai(agent) => agent.clone(),
        seat => panic!("seat zero is not an AI chair: {seat:?}"),
    };

    let dealt = seated(41);
    let here = chair(&dealt);
    // One other seed would almost always agree by luck: the noise only
    // decides when two scores are equal, and when it does it still has to
    // land on the other side. The unit test for the same tie sweeps 32
    // seeds for that reason, and so does this.
    let elsewhere: Vec<HeuristicAgent> = (0..32).map(|s| chair(&seated(s))).collect();

    let mut engine = Engine::new(&dealt, RegistryLookup).unwrap();
    let mut asked = 0usize;
    for seq in 0..120 {
        let Some(seat) = pending_player(engine.pending()) else {
            break;
        };
        let view = asked_view(engine.state(), seat, seq, engine.pending());
        if view.turn > 2 {
            break;
        }
        let action = match engine.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            _ if seat == PlayerId::new(0) => {
                let mine = here.act(&view, engine.pending());
                for (seed, chair) in elsewhere.iter().enumerate() {
                    assert_eq!(
                        mine,
                        chair.act(&view, engine.pending()),
                        "the same question, answered differently by a chair from \
                         a game that differs only in the seed its cards were \
                         dealt from (seed {seed}, question {asked}, seq {seq})"
                    );
                }
                asked += 1;
                mine
            }
            Pending::Priority { .. } => PlayerAction::PassPriority,
            // The opponent is a post, not a player: it swings with nothing
            // and blocks with nothing, so the only thing moving between the
            // two runs is the chair under test.
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers {
                attackers: Vec::new(),
            },
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers {
                blockers: Vec::new(),
            },
            other => panic!("unexpected opposing question: {other:?}"),
        };
        engine
            .apply(seat, action)
            .expect("every answer came out of the offer");
    }
    assert!(
        asked >= 8,
        "the premise: this fixture has to reach the tie often enough to be \
         able to disagree, and it asked only {asked} questions"
    );
}

/// #87, the other half: the chair still *has* randomness, and it is the
/// table's own.
///
/// The test above is satisfied by an agent with no randomness at all, and by
/// a `describe` that does nothing — every chair would share the
/// no-identifier derivation and agree for that reason instead of the right
/// one. So the same tie is asked of chairs from one preset described under
/// thirty-two different public game identifiers, and they must not all
/// answer alike. That is the counter-test for the assertion above and the
/// only thing that says the identifier reaches the chair at all.
#[test]
fn a_described_table_breaks_its_ties_with_its_own_randomness() {
    let mut preset = position(&["Brainstorm", "Brainstorm"], &["Island", "Island"]);
    preset.seats[0].controller = SeatController::Ai(AIProfile::EXPERT);
    let chair = |game: &str| {
        let mut session = Session::new(&preset).expect("the fixture builds a session");
        session.describe(game.to_string(), Vec::new());
        match session
            .seat_kind(PlayerId::new(0))
            .expect("seat zero is at the table")
        {
            SeatKind::Ai(agent) => agent.clone(),
            seat => panic!("seat zero is not an AI chair: {seat:?}"),
        }
    };
    let tables: Vec<HeuristicAgent> = (0..32).map(|i| chair(&format!("game-{i}"))).collect();

    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let mut differed = false;
    let mut asked = 0usize;
    for seq in 0..120 {
        let Some(seat) = pending_player(engine.pending()) else {
            break;
        };
        let view = asked_view(engine.state(), seat, seq, engine.pending());
        if view.turn > 2 {
            break;
        }
        let action = match engine.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            _ if seat == PlayerId::new(0) => {
                let answers: Vec<PlayerAction> = tables
                    .iter()
                    .map(|t| t.act(&view, engine.pending()))
                    .collect();
                differed |= answers.iter().any(|a| *a != answers[0]);
                asked += 1;
                answers[0].clone()
            }
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers {
                attackers: Vec::new(),
            },
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers {
                blockers: Vec::new(),
            },
            other => panic!("unexpected opposing question: {other:?}"),
        };
        engine
            .apply(seat, action)
            .expect("every answer came out of the offer");
    }
    assert!(asked >= 8, "the premise: only {asked} questions were asked");
    assert!(
        differed,
        "thirty-two tables, one tie, and every chair answered it the same \
         way: either the identifier never reaches the chair or the chair has \
         no randomness left for the test above to be about"
    );
}

#[test]
fn a_planned_multicolour_cast_survives_the_mana_choice_round_trip() {
    let preset = position(
        &[
            "Baleful Strix",
            "Loran of the Third Path",
            "Loran of the Third Path",
            "Loran of the Third Path",
            "Loran of the Third Path",
        ],
        &["City of Brass", "Swamp"],
    );
    for profile in [AIProfile::STEADY, AIProfile::SHARP, AIProfile::EXPERT] {
        let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
        let agent = HeuristicAgent::new(profile);
        let mut cast = false;
        for seq in 0..100 {
            let Some(seat) = pending_player(engine.pending()) else {
                break;
            };
            let view = asked_view(engine.state(), seat, seq, engine.pending());
            if view.turn > 1 {
                break;
            }
            cast |= view.battlefield.iter().any(|o| {
                o.card
                    .is_some_and(|c| c.index == entry("Baleful Strix").card)
            });
            let action = match engine.pending() {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                _ if seat == PlayerId::new(0) => agent.act(&view, engine.pending()),
                Pending::Priority { .. } => PlayerAction::PassPriority,
                _ => panic!("unexpected opposing question: {:?}", engine.pending()),
            };
            engine
                .apply(seat, action)
                .expect("every planned action is legal");
        }
        assert!(
            cast,
            "{profile:?}: two untapped sources must cast Strix despite the white-heavy hand"
        );
    }
}

#[test]
fn the_ai_casts_both_commanders_with_independent_cast_counts() {
    // Freeform setup exercises the multi-commander engine path without
    // claiming these two cards form a legal Partner pair. Real pairing
    // validation fixtures are tracked in docs/ai-coverage-todo.md.
    let mut preset = position(
        &[],
        &["Forest", "Plains", "Island", "Swamp", "Plains", "Island"],
    );
    preset.seats[0].commanders = vec![
        entry("Katara, the Fearless"),
        entry("Aminatou, the Fateshifter"),
    ];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    for seq in 0..150 {
        let pending = engine.pending();
        let seat = pending_player(pending).expect("fixture remains live");
        let view = asked_view(engine.state(), seat, seq, pending);
        if view
            .battlefield
            .iter()
            .filter(|o| o.commander && o.controller == PlayerId::new(0))
            .count()
            == 2
        {
            assert_eq!(
                engine.state().commanders[0]
                    .iter()
                    .map(|c| c.casts)
                    .collect::<Vec<_>>(),
                vec![1, 1]
            );
            return;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        engine
            .apply(seat, action)
            .expect("each commander's payment and choice are legal");
    }
    panic!("six correctly coloured lands must deploy both three-mana commanders");
}

#[test]
fn an_x_draw_spell_pays_for_x_and_draws_for_its_caster() {
    let preset = position(
        &["Commander's Insight"],
        &["Island", "Island", "Island", "Island", "Island"],
    );
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let mut chosen_x = false;
    for seq in 0..100 {
        let pending = engine.pending();
        let seat = pending_player(pending).unwrap();
        let view = asked_view(engine.state(), seat, seq, pending);
        if chosen_x && view.hand.len() == 2 && seat == PlayerId::new(0) {
            assert_eq!(view.seats[1].hand_count, 0);
            return;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        if matches!(pending, Pending::ChooseNumber { .. }) {
            assert_eq!(action, PlayerAction::ChooseNumber(2));
            chosen_x = true;
        }
        if matches!(pending, Pending::ChoosePlayer { .. }) {
            assert_eq!(
                action,
                PlayerAction::ChoosePlayer(PlayerId::new(0)),
                "draw effects belong to the caster"
            );
        }
        engine
            .apply(seat, action)
            .expect("X mana and target are legal");
    }
    panic!("the two-card draw must resolve");
}

#[test]
fn x_cannot_demand_more_graveyard_targets_than_exist() {
    let preset = position(
        &["Entreat the Dead"],
        &["Swamp", "Swamp", "Swamp", "Swamp", "Swamp"],
    );
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    for seq in 0..80 {
        let pending = engine.pending();
        let seat = pending_player(pending).unwrap();
        let view = asked_view(engine.state(), seat, seq, pending);
        if view.turn > 1 {
            return;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        if matches!(pending, Pending::ChooseNumber { .. }) {
            assert_eq!(action, PlayerAction::ChooseNumber(0));
        }
        engine
            .apply(seat, action)
            .expect("no under-targeted X cast may roll back");
    }
    panic!("the decision must make progress");
}

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
            }])),
            ..CardDef::DEFAULT
        }));
        let object = |index| DeckEntry {
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
