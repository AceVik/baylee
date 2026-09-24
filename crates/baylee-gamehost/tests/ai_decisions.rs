//! Decisions tested across the real engine/view boundary.

use baylee_ai::{AIProfile, HeuristicAgent, pending_player};
use baylee_core::ids::{ObjectId, PlayerId};
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

/// #166 / #170. The pool's first card to put a time counter on anything is
/// a land the agent cannot see as a land.
///
/// Trenzalore Clocktower is `{T}: Add {U}. Put a time counter on Trenzalore
/// Clocktower.`, and #166 was opened on the worry that the counter would be
/// *scored* backwards — a time counter is a delay on a suspended card, and
/// this one is a private count to twelve that its controller wants. It is
/// not scored at all, backwards or otherwise: the `AddCounter` sits inside
/// the mana ability with no target, so nothing ever asks `clock_score` about
/// it. The unit test beside `a_time_counter_delays_the_suspended_card_that_is_about_to_cast`
/// holds the scoring half.
///
/// What the card actually does is worse and is **#170**: `mana_shape` matches
/// a one-element `[Effect::AddMana { .. }]`, so a mana ability with any second
/// sentence is invisible to the planner. The Clocktower is one of 92 such
/// faces — every painland, every Karoo, every Odyssey filter land, the
/// Talismans and Signets, Ancient Tomb.
///
/// So this is a **pinned limitation** and it is written to fail the day it is
/// fixed. The first assertion is the one that keeps it honest: the engine
/// *does* offer the ability, so what follows is the agent declining an offer
/// rather than a board that never had one. Without it, deleting the
/// Clocktower from the fixture would leave the other two assertions green.
#[test]
fn a_mana_land_that_also_counts_is_invisible_to_the_planner() {
    let mut offered = false;
    let mut pressed = false;
    let mut cast = false;
    let mut control_cast = false;
    for land in ["Trenzalore Clocktower", "Island"] {
        let preset = position(&["Brainstorm", "Brainstorm"], &[land, land]);
        let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
        let agent = HeuristicAgent::new(AIProfile::EXPERT);
        for seq in 0..100 {
            let Some(seat) = pending_player(engine.pending()) else {
                break;
            };
            let view = asked_view(engine.state(), seat, seq, engine.pending());
            if view.turn > 1 {
                break;
            }
            let ours = |id| {
                view.object(id)
                    .and_then(|o| o.card)
                    .is_some_and(|c| c.index == entry(land).card)
            };
            if let Pending::Priority { legal, .. } = engine.pending()
                && seat == PlayerId::new(0)
                && legal.abilities.iter().any(|(id, _)| ours(*id))
                && land == "Trenzalore Clocktower"
            {
                offered = true;
            }
            let action = match engine.pending() {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                _ if seat == PlayerId::new(0) => agent.act(&view, engine.pending()),
                Pending::Priority { .. } => PlayerAction::PassPriority,
                other => panic!("unexpected opposing question: {other:?}"),
            };
            if seat == PlayerId::new(0) {
                match &action {
                    PlayerAction::ActivateManaAbility { source, .. }
                    | PlayerAction::ActivateAbility { source, .. }
                        if ours(*source) && land == "Trenzalore Clocktower" =>
                    {
                        pressed = true;
                    }
                    PlayerAction::CastSpell { .. } if land == "Trenzalore Clocktower" => {
                        cast = true;
                    }
                    PlayerAction::CastSpell { .. } => control_cast = true,
                    _ => {}
                }
            }
            engine
                .apply(seat, action)
                .expect("every planned action is legal");
        }
    }
    assert!(
        control_cast,
        "the control is the measurement: two Islands and two Brainstorms must \
         produce a cast, or this test says nothing about the Clocktower"
    );
    assert!(
        offered,
        "the engine must offer the Clocktower's mana ability, or what follows \
         is a board with nothing to press rather than an agent declining"
    );
    assert!(
        !pressed && !cast,
        "#170 is fixed: the planner now reads a mana ability that has a second \
         sentence. Delete this test and assert the cast instead."
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

/// #168. A permanent that prints a free tap and a priced one is ranked by
/// what the mana **costs**, not only by how much of it there is.
///
/// `policy::sources` collapses one permanent to one `manaplan::Source`,
/// because nothing under `manaplan::plan` keys on `ObjectId` and two entries
/// with one id would let the solver tap the same land twice. So the entry
/// that survives the dedup is the *only* mode the agent will ever use for
/// that permanent — and the key ranked on mana made, then colours reached,
/// and never on the price.
///
/// Two shapes, one per column of that key:
///
/// - **Havenwood Battleground** — `{T}: Add {G}` beside `{T}, Sacrifice this
///   land: Add {G}{G}`. More mana won, so the agent sold the land for a
///   green it already had.
/// - **Spire of Industry** — `{T}: Add {C}` beside `{T}, Pay 1 life: Add one
///   mana of any color`. Equal amounts, five colours against one, so it paid
///   the life even when colourless was the whole of what the plan asked for.
///
/// The assertion is the **ability index** rather than the board, because a
/// board says what survived and an index says which button was pressed: a
/// Havenwood still in play could equally mean the agent never tapped it.
/// `pressed` being non-empty is what rules that out, and it is the assertion
/// that would catch a fixture whose land came in tapped.
#[test]
fn a_permanent_with_a_free_and_a_priced_tap_is_ranked_by_what_it_costs() {
    for (board, spell, land) in [
        (
            vec!["Havenwood Battleground"],
            "Llanowar Elves",
            "Havenwood Battleground",
        ),
        (
            vec!["Spire of Industry", "Mox Opal"],
            "Sol Ring",
            "Spire of Industry",
        ),
    ] {
        let preset = position(&[spell], &board);
        let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
        let agent = HeuristicAgent::new(AIProfile::EXPERT);
        let mut pressed: Vec<u32> = Vec::new();
        let mut cast = false;
        for seq in 0..100 {
            let Some(seat) = pending_player(engine.pending()) else {
                break;
            };
            let view = asked_view(engine.state(), seat, seq, engine.pending());
            if view.turn > 1 {
                break;
            }
            let is_land = |id| {
                view.object(id)
                    .and_then(|o| o.card)
                    .is_some_and(|c| c.index == entry(land).card)
            };
            let action = match engine.pending() {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                _ if seat == PlayerId::new(0) => agent.act(&view, engine.pending()),
                Pending::Priority { .. } => PlayerAction::PassPriority,
                other => panic!("unexpected opposing question: {other:?}"),
            };
            if seat == PlayerId::new(0) {
                match &action {
                    PlayerAction::ActivateAbility {
                        source,
                        ability_index,
                    } if is_land(*source) => pressed.push(*ability_index),
                    PlayerAction::CastSpell { .. } => cast = true,
                    _ => {}
                }
            }
            engine
                .apply(seat, action)
                .expect("every planned action is legal");
        }
        assert!(
            cast,
            "{land}: the agent never cast {spell}, so the plan this \
             test is about was never made"
        );
        assert!(
            !pressed.is_empty(),
            "{land}: the agent never tapped it at all, so the ranking below \
             is being read off an empty list"
        );
        assert!(
            pressed.iter().all(|index| *index == 0),
            "{land}: the agent pressed {pressed:?}, and index 0 is the free \
             mode. A priced mode that wins the dedup is the only mode there \
             is for that permanent"
        );
    }
}

/// Answers everything a hand-driven fixture does not ask about and stops at
/// seat 0's own main phase with the stack empty. A target question is
/// answered with whichever of `aim` is among its options.
fn settle(engine: &mut Engine<RegistryLookup>, aim: &[ObjectId]) {
    let me = PlayerId::new(0);
    for _ in 0..100 {
        let pending = engine.pending().clone();
        let Some(seat) = pending_player(&pending) else {
            panic!("the game ended");
        };
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat != me => PlayerAction::PassPriority,
            Pending::Priority { .. } => {
                let view = asked_view(engine.state(), me, 0, engine.pending());
                if view.stack.is_empty()
                    && view.active == me
                    && view.phase == baylee_view::Phase::FirstMain
                {
                    return;
                }
                PlayerAction::PassPriority
            }
            Pending::ChooseTargets { options, .. } => PlayerAction::ChooseObjects {
                objects: options.into_iter().filter(|o| aim.contains(o)).collect(),
            },
            Pending::YesNo { .. } => PlayerAction::YesNo(true),
            other => panic!("unexpected question: {other:?}"),
        };
        engine.apply(seat, action).expect("a legal answer");
    }
    panic!("the table never came back to seat 0's main phase");
}

/// Seat 0's untapped permanents whose card is `name`.
fn untapped(view: &PlayerView, name: &str) -> Vec<ObjectId> {
    view.battlefield_of(PlayerId::new(0))
        .filter(|o| o.card.is_some_and(|c| c.index == entry(name).card))
        .filter(|o| !o.status.contains(baylee_view::ObjectStatus::TAPPED))
        .map(|o| o.id)
        .collect()
}

/// Taps each of seat 0's `sources` for mana, which then floats.
fn float(engine: &mut Engine<RegistryLookup>, sources: &[ObjectId]) {
    for &source in sources {
        engine
            .apply(
                PlayerId::new(0),
                PlayerAction::ActivateManaAbility { source },
            )
            .expect("a basic land taps for mana");
    }
}

/// #214. A copy uses the abilities of the card it copied (CR 707.2).
///
/// The engine offers a copy's ability as an index into the copied card's
/// list, and the agent looked that index up on the card underneath: a
/// Glasspool Mimic that entered as a Werefox Bodyguard prints one ability of
/// its own, so the Fox's second came back as nothing and the copy was never
/// used. The view names the copied card in `rules` since `VIEW_VERSION` 28,
/// and this board is the one `combo_tests/printed_faces.rs` plays.
///
/// Two things are arranged by hand so the question is the only one on the
/// table. The original Fox is sacrificed first, because `activate::useful`
/// takes the first offer it recognises and the original, recognised either
/// way, comes first. And the `{1}{W}` is floated before the agent is asked,
/// because the engine offers an activation only against mana already in the
/// pool — without it the Mimic's ability is not on offer at all, and a pass
/// would say nothing about the agent.
///
/// Sacrificing a 2/2 for 2 life is the whitelist's answer, not a judgement
/// this test defends. If `activate::useful` stops taking it, this goes red
/// for a reason that is not #214; the two `a_copy_*` tests in `baylee-ai`
/// are the ones that carry the lookup.
#[test]
fn a_copy_uses_the_ability_of_the_card_it_copied() {
    let me = PlayerId::new(0);
    let preset = position(
        &["Glasspool Mimic"],
        &[
            "Werefox Bodyguard",
            "Island",
            "Island",
            "Island",
            "Plains",
            "Plains",
            "Plains",
            "Plains",
        ],
    );
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();

    settle(&mut engine, &[]);
    let view = asked_view(engine.state(), me, 1, engine.pending());
    let fox = untapped(&view, "Werefox Bodyguard")[0];
    let mimic = view
        .hand
        .iter()
        .find(|c| c.card.index == entry("Glasspool Mimic").card)
        .expect("the Mimic in hand")
        .id;
    float(&mut engine, &untapped(&view, "Island"));
    engine
        .apply(me, PlayerAction::CastSpell { card: mimic })
        .expect("three Islands pay for the Mimic");
    // The Fox for the Mimic's copy, and nothing for the copied enters
    // trigger, which may exile only a creature that is not a Fox.
    settle(&mut engine, &[fox]);

    let view = asked_view(engine.state(), me, 2, engine.pending());
    let copy = untapped(&view, "Glasspool Mimic")[0];
    assert_eq!(
        view.object(copy).and_then(|o| o.rules).map(|r| r.card),
        Some(entry("Werefox Bodyguard").card),
        "the Mimic did not arrive as a copy of the Fox, so there is no copy \
         for the agent to read"
    );
    float(&mut engine, &untapped(&view, "Plains")[..2]);
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: fox,
                ability_index: 1,
            },
        )
        .expect("the original Fox sacrifices itself");
    settle(&mut engine, &[]);

    let view = asked_view(engine.state(), me, 3, engine.pending());
    float(&mut engine, &untapped(&view, "Plains")[..2]);
    let view = asked_view(engine.state(), me, 4, engine.pending());
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert_eq!(
        legal.abilities,
        vec![(copy, 1)],
        "the engine must offer the copy the Fox's sacrifice and nothing else, \
         or what follows is not this question"
    );
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act(&view, engine.pending()),
        PlayerAction::ActivateAbility {
            source: copy,
            ability_index: 1,
        },
        "the agent looked the Fox's ability up on the Mimic underneath"
    );
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
                second_targets: None,
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
