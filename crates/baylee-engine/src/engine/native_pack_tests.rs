//! Compiled admission, setup-only placement and ordinary rule execution.
use super::*;
use crate::extensions::{NativeDefinition, NativeLookup, NativeRulePack};
use crate::state::SetupError;
use baylee_cards_dsl::{Effect, PlayerRel, StepKind, TokenDef, Trigger, triggered};
use baylee_core::{
    ids::CardIndex,
    preset::{LoopPolicy, RulePackSetup, SeatExtensions},
};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

static SOURCE: TokenDef = TokenDef {
    name: "Synthetic native source",
    types: baylee_core::types::TypeSet::ARTIFACT,
    abilities: &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::gain_life(2)]
        ),
        triggered!(
            Trigger::EntersBattlefield(&baylee_cards_dsl::Filter::This),
            &[Effect::gain_life(9)]
        ),
    ],
    ..TokenDef::DEFAULT
};

#[derive(Clone, Copy, Default)]
struct Registry;
impl CardLookup for Registry {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
    fn token_id(&self, token: &TokenDef) -> Option<u16> {
        std::ptr::eq(token, &raw const SOURCE).then_some(0)
    }
}

#[derive(Clone)]
struct Pack;
impl NativeRulePack for Pack {
    const PACK: u16 = 1;
    const REVISION: u16 = 2;
    fn validate(preset: &GamePreset) -> Result<(), SetupError> {
        if preset.house_rules.loop_policy == LoopPolicy::Off {
            Ok(())
        } else {
            Err(SetupError::UnsupportedRulePack)
        }
    }
    fn definition(id: u16) -> Option<NativeDefinition> {
        match id {
            1 => Some(NativeDefinition::Permanent(&SOURCE)),
            2 => Some(NativeDefinition::Emblem(&SOURCE)),
            3 | 4 => Some(NativeDefinition::SetupOnly),
            _ => None,
        }
    }
}

fn preset(definitions: Vec<u16>) -> GamePreset {
    let mut preset = super::opening_rules_tests::preset(Vec::new());
    for seat in &mut preset.seats {
        seat.starting_hand = Some(Vec::new());
    }
    preset.house_rules.loop_policy = LoopPolicy::Off;
    preset.house_rules.rule_pack = Some(RulePackSetup {
        pack: Pack::PACK,
        revision: Pack::REVISION,
        seats: vec![SeatExtensions {
            seat: 0,
            definitions,
        }],
    });
    preset
}

type Packed = Engine<NativeLookup<Registry, Pack>, NoLoopDetection>;
fn game(preset: &GamePreset) -> Result<Packed, EngineError> {
    Packed::with_detection(preset, NativeLookup::default())
}

#[test]
fn native_pack_admission_fails_closed_before_constructing_a_game() {
    let valid = preset(vec![1, 2]);
    assert!(Engine::<Registry, NoLoopDetection>::with_detection(&valid, Registry).is_err());
    assert!(game(&valid).is_ok());
    let mut missing = valid.clone();
    missing.house_rules.rule_pack = None;
    assert!(game(&missing).is_err());
    let mut wrong = valid.clone();
    wrong.house_rules.rule_pack.as_mut().unwrap().pack += 1;
    assert!(game(&wrong).is_err());
    wrong = valid.clone();
    wrong.house_rules.rule_pack.as_mut().unwrap().revision += 1;
    assert!(game(&wrong).is_err());
    assert!(game(&preset(vec![1, u16::MAX])).is_err());
    wrong = valid;
    wrong.house_rules.loop_policy = LoopPolicy::RunOnceThenBreak;
    assert!(Engine::<NativeLookup<Registry, Pack>>::new(&wrong, NativeLookup::default()).is_err());
}

#[test]
fn native_setup_sources_trigger_at_real_upkeep_and_preserve_the_replay_hash() {
    let preset = preset(vec![1, 2]);
    let mut a = game(&preset).unwrap();
    let mut b = game(&preset).unwrap();
    assert_eq!(a.state.zones.list(ZoneLocation::Battlefield).len(), 1);
    assert_eq!(a.state.zones.list(ZoneLocation::Command(P0)).len(), 1);
    assert!(a.state.zones.list(ZoneLocation::Command(P1)).is_empty());
    for (player, action) in [
        (P0, PlayerAction::MulliganKeep),
        (P1, PlayerAction::MulliganKeep),
    ] {
        a.apply(player, action.clone()).unwrap();
        b.apply(player, action).unwrap();
        assert_eq!(a.snapshot_hash(), b.snapshot_hash());
    }
    assert_eq!(
        a.state.players[0].life, 20,
        "setup does not invent entry triggers"
    );
    assert_eq!(
        a.state.zones.list(ZoneLocation::Stack).len(),
        2,
        "both native upkeeps use the real stack"
    );
    for _ in 0..12 {
        if a.state.zones.list(ZoneLocation::Stack).is_empty() {
            break;
        }
        let Pending::Priority { player, .. } = *a.pending() else {
            panic!("expected priority");
        };
        a.apply(player, PlayerAction::PassPriority).unwrap();
        b.apply(player, PlayerAction::PassPriority).unwrap();
        assert_eq!(a.snapshot_hash(), b.snapshot_hash());
    }
    assert_eq!(a.state.players[0].life, 24);
    assert_eq!(a.state.players[1].life, 20);
}

#[test]
fn native_pack_has_no_adapter_storage_and_setup_only_ids_remain_in_the_hash() {
    assert_eq!(std::mem::size_of::<NativeLookup<Registry, Pack>>(), 0);
    assert_eq!(
        std::mem::size_of::<Packed>(),
        std::mem::size_of::<Engine<Registry, NoLoopDetection>>()
    );
    let a = game(&preset(vec![3])).unwrap();
    let b = game(&preset(vec![4])).unwrap();
    assert_eq!(a.state.snapshot_hash(), b.state.snapshot_hash());
    assert_ne!(a.snapshot_hash(), b.snapshot_hash());
    let mut ordinary = preset(vec![]);
    ordinary.house_rules.rule_pack = None;
    let original =
        Engine::<Registry, NoLoopDetection>::with_detection(&ordinary, Registry).unwrap();
    let round_trip = serde_json::from_slice(&serde_json::to_vec(&ordinary).unwrap()).unwrap();
    let restored =
        Engine::<Registry, NoLoopDetection>::with_detection(&round_trip, Registry).unwrap();
    assert_eq!(original.snapshot_hash(), restored.snapshot_hash());
}

#[test]
fn a_native_source_without_a_shared_presentation_identity_is_refused_before_card_lookup() {
    struct MissingIdentity;
    impl CardLookup for MissingIdentity {
        fn card(&self, _: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            panic!("admission must fail before building a deck");
        }
    }
    let result = Engine::<_, NoLoopDetection>::with_detection(
        &preset(vec![1]),
        NativeLookup::<_, Pack>::new(MissingIdentity),
    );
    assert!(matches!(
        result,
        Err(EngineError::Setup(SetupError::UnsupportedRulePack))
    ));
}

#[test]
fn removing_a_native_permanent_keeps_its_pending_trigger_but_stops_future_ones() {
    let mut engine = game(&preset(vec![1, 2])).unwrap();
    engine.apply(P0, PlayerAction::MulliganKeep).unwrap();
    engine.apply(P1, PlayerAction::MulliganKeep).unwrap();
    let source = engine.state.zones.list(ZoneLocation::Battlefield)[0];
    crate::sba::put_into_graveyard(&mut engine.state, source);
    for _ in 0..100 {
        if engine.state.turn.number >= 3 && engine.state.turn.phase == Phase::FirstMain {
            break;
        }
        let (player, action) = match engine.pending().clone() {
            Pending::Priority { player, .. } => (player, PlayerAction::PassPriority),
            Pending::ChooseAttackers { player, .. } => {
                (player, PlayerAction::DeclareAttackers { attackers: vec![] })
            }
            Pending::ChooseBlockers { player, .. } => {
                (player, PlayerAction::DeclareBlockers { blockers: vec![] })
            }
            pending => panic!("unexpected fixture choice: {pending:?}"),
        };
        engine.apply(player, action).unwrap();
    }
    assert_eq!(engine.state.turn.number, 3);
    assert_eq!(engine.state.turn.phase, Phase::FirstMain);
    assert_eq!(
        engine.state.players[0].life, 26,
        "two captured triggers, then only the emblem"
    );
    assert!(
        !engine
            .state
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&source)
    );
}
