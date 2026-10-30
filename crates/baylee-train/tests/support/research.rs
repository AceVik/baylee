//! Real-engine research fixtures. Logged probabilities are synthetic test data;
//! these fixtures exercise structural conversion, not numerical policy admission.
use super::train;
use baylee_core::{
    ids::{ObjectId, PlayerId, PrintRef},
    preset::{AIProfile, DeckEntry},
};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use serde_json::{Value, json};
use train::{
    convert4::RolloutStepSidecar4,
    failure_terminal,
    features4::{HeadKind4, SourceKind4, StepProvenance4, encode_executed_action4},
    features7::{encode_candidates7_grouped, encode_state7},
};

pub const OWNER: PlayerId = PlayerId::new(0);
pub const OTHER: PlayerId = PlayerId::new(1);

pub struct Fixture {
    pub record: Vec<u8>,
    pub sidecar: Vec<RolloutStepSidecar4>,
    pub episode: Value,
}

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::all()
            .find(|card| card.name() == name)
            .unwrap()
            .index,
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

fn act(
    session: &mut Session,
    sidecar: &mut Vec<RolloutStepSidecar4>,
    bindings: &mut Vec<Value>,
    action: PlayerAction,
) {
    let seat = session.pending().asked().unwrap();
    let (pending, view) = session.view_for_with_context(seat).unwrap();
    let state = encode_state7(&view, Some(&pending), None, &[None, None]);
    let menu = encode_candidates7_grouped(&state, &view, &pending);
    let forced = match menu.head {
        HeadKind4::SingleChoice => menu.candidates.len() <= 1,
        HeadKind4::SubsetPick => menu.candidates.is_empty(),
        HeadKind4::PairPick => menu.pairs.is_empty(),
        HeadKind4::ScalarValue => menu.max_choices <= menu.min_choices,
    };
    let target = encode_executed_action4(&menu, &pending, &action).unwrap_or_default();
    let actor = !forced && menu.head != HeadKind4::ScalarValue;
    let step = RolloutStepSidecar4 {
        provenance: StepProvenance4 {
            source: if forced {
                SourceKind4::Forced
            } else {
                SourceKind4::Net
            },
            has_logprob: actor,
            temperature: 0.7,
            thinking_steps: if forced { 0 } else { 2 },
            behavior_logprob: if actor { -0.7 } else { 0.0 },
        },
        free_mask: target.free_mask,
        pick_sequence: target.pick_sequence,
    };
    let mut binding = failure_terminal::sample_binding(session, seat, 7, &action, &step).unwrap();
    session.act_slice(seat, action, 0).unwrap();
    binding["post_snapshot_hash"] = json!(format!("{:016x}", session.snapshot_hash()));
    sidecar.push(step);
    bindings.push(binding);
}

fn setup(natural: bool) -> (Session, Vec<RolloutStepSidecar4>, Vec<Value>, ObjectId) {
    let deck = train::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset = train::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.seats[0].starting_hand = Some(vec![entry("Fireball")]);
    preset.seats[0].starting_battlefield = if natural {
        vec![entry("Mountain"), entry("Wastes")]
    } else {
        vec![entry("Mountain")]
    };
    preset.seats[1].starting_hand = Some(vec![entry("Forest")]);
    preset.seats[1].starting_life = Some(if natural { 1 } else { 20 });
    let mut session = Session::new_recorded(&preset, "research-conversion-test").unwrap();
    for player in [OWNER, OTHER] {
        session.take_over(player);
    }
    let (mut sidecar, mut bindings) = (Vec::new(), Vec::new());
    for _ in 0..64 {
        if session.observe_view(OWNER).phase == baylee_view::Phase::FirstMain
            && matches!(session.pending(),Pending::Priority {player,..} if *player==OWNER)
        {
            break;
        }
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected fixture opening {other:?}"),
        };
        act(&mut session, &mut sidecar, &mut bindings, action);
    }
    let sources: Vec<_> = session
        .observe_view(OWNER)
        .battlefield
        .iter()
        .map(|object| object.id)
        .collect();
    for source in sources {
        let action = if matches!(session.pending(),Pending::Priority {legal,..} if legal.mana_abilities.contains(&source))
        {
            PlayerAction::ActivateManaAbility { source }
        } else {
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            }
        };
        act(&mut session, &mut sidecar, &mut bindings, action);
    }
    let card = session
        .observe_view(OWNER)
        .hand
        .iter()
        .find(|card| card.name == "Fireball")
        .unwrap()
        .id;
    (session, sidecar, bindings, card)
}

fn cast(
    session: &mut Session,
    sidecar: &mut Vec<RolloutStepSidecar4>,
    bindings: &mut Vec<Value>,
    card: ObjectId,
    x: u32,
) {
    act(session, sidecar, bindings, PlayerAction::CastSpell { card });
    while matches!(session.pending(), Pending::ChooseCastMode { .. }) {
        act(session, sidecar, bindings, PlayerAction::ChooseMode(0));
    }
    act(session, sidecar, bindings, PlayerAction::ChooseNumber(x));
}

pub fn loop_episode(learner: usize) -> Fixture {
    let (mut session, mut sidecar, mut bindings, card) = setup(false);
    for _ in 0..3 {
        cast(&mut session, &mut sidecar, &mut bindings, card, 0);
        act(
            &mut session,
            &mut sidecar,
            &mut bindings,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![OWNER, OTHER],
            },
        );
    }
    let hash = failure_terminal::boundary_view_hash(&session, OWNER, 7).unwrap();
    let snapshot = session.snapshot_hash();
    let record = session.take_record();
    let episode = failure_terminal::stop(
        &record,
        &sidecar,
        &bindings,
        7,
        learner,
        OWNER,
        true,
        "internal_repeated_view",
        hash,
        Some(&PlayerAction::PassPriority),
        snapshot,
    )
    .unwrap();
    Fixture {
        record,
        sidecar,
        episode,
    }
}

pub fn natural_episode(learner: u8) -> Fixture {
    let (mut session, mut sidecar, mut bindings, card) = setup(true);
    cast(&mut session, &mut sidecar, &mut bindings, card, 1);
    act(
        &mut session,
        &mut sidecar,
        &mut bindings,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![OTHER],
        },
    );
    for _ in 0..32 {
        if matches!(session.pending(), Pending::GameOver(_)) {
            break;
        }
        act(
            &mut session,
            &mut sidecar,
            &mut bindings,
            PlayerAction::PassPriority,
        );
    }
    assert!(matches!(session.pending(), Pending::GameOver(_)));
    let record = session.take_record();
    let episode = failure_terminal::natural(
        &json!({"kind":"won","seat":0}),
        if learner == 0 { 1.0 } else { 0.0 },
        &bindings,
    );
    Fixture {
        record,
        sidecar,
        episode,
    }
}
