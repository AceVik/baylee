//! Failures after automatic effects restore the complete published decision.
use super::synthetic::{self, SyntheticLookup};
use super::*;
use baylee_cards_dsl::prelude::*;
use baylee_core::mana::ManaColor;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);
const SOURCE: u32 = 995_120;

fn fixture(abilities: &'static [AbilityDef]) -> Engine<SyntheticLookup> {
    let mut engine = Engine::new(
        &synthetic::preset(909_712, &[SOURCE]),
        SyntheticLookup::new(vec![synthetic::land(SOURCE, "Capacity probe", abilities)]),
    )
    .unwrap();
    synthetic::keep_mulligans(&mut engine);
    for _ in 0..20 {
        if matches!(engine.pending(), Pending::Priority { player: ME, .. }) {
            return engine;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending), "{pending:?}");
    }
    panic!("no priority");
}

#[test]
fn mana_production_overflow_refuses_the_tap_and_journal_atomically() {
    static ABILITIES: &[AbilityDef] = &[activated!(
        Cost::TAP,
        &[Effect::AddMana {
            source: ManaSource::Fixed(ManaColor::Blue),
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None
        }],
        mana_ability = true
    )];
    let mut engine = fixture(ABILITIES);
    let source = synthetic::permanents(&engine, SOURCE)[0];
    engine.state.players[0]
        .mana_pool
        .add(ManaColor::Blue, u32::MAX);
    let action = PlayerAction::ActivateAbility {
        source,
        ability_index: 0,
    };
    let before = engine.fingerprint();
    assert!(matches!(
        engine.apply(ME, action.clone()),
        Err(EngineError::NumericCapacity(_))
    ));
    assert_eq!(engine.fingerprint(), before);
    assert!(!synthetic::tapped(&engine, source));
    assert!(engine.state.players[0].mana_pool.spend(ManaColor::Blue, 1));
    engine.apply(ME, action).unwrap();
    assert!(synthetic::tapped(&engine, source));
    assert_eq!(
        engine.state.players[0].mana_pool.available(ManaColor::Blue),
        u32::MAX
    );
}

#[test]
fn a_resolving_transfer_overflow_keeps_both_pools_and_its_stack_continuation() {
    static ABILITIES: &[AbilityDef] = &[activated!(
        Cost::FREE,
        &[Effect::ActivateLandsAndTakeMana {
            player: PlayerRel::Chosen
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    )];
    let mut engine = fixture(ABILITIES);
    let source = synthetic::permanents(&engine, SOURCE)[0];
    engine
        .apply(
            ME,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    match engine.pending() {
        Pending::ChoosePlayer { .. } => engine.apply(ME, PlayerAction::ChoosePlayer(THEM)).unwrap(),
        Pending::ChooseTargets { .. } => engine
            .apply(
                ME,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![THEM],
                },
            )
            .unwrap(),
        pending => panic!("target: {pending:?}"),
    }
    engine.state.players[0]
        .mana_pool
        .add(ManaColor::Blue, u32::MAX);
    engine.state.players[1].mana_pool.add(ManaColor::Blue, 1);
    for _ in 0..5 {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("priority expected");
        };
        let before = engine.fingerprint();
        match engine.apply(player, PlayerAction::PassPriority) {
            Ok(()) => {}
            Err(EngineError::NumericCapacity(_)) => {
                assert_eq!(engine.fingerprint(), before);
                assert_eq!(
                    engine.state.players[1].mana_pool.available(ManaColor::Blue),
                    1
                );
                assert!(engine.state.players[0].mana_pool.spend(ManaColor::Blue, 1));
                engine.apply(player, PlayerAction::PassPriority).unwrap();
                assert_eq!(
                    engine.state.players[0].mana_pool.available(ManaColor::Blue),
                    u32::MAX
                );
                assert!(engine.state.players[1].mana_pool.is_empty());
                assert!(engine.state.zones.stack_is_empty());
                return;
            }
            Err(error) => panic!("unexpected: {error:?}"),
        }
    }
    panic!("transfer never rejected");
}

const FREE: u32 = 995_121;
const FILTER: u32 = 995_122;
const PHASED: u32 = 995_123;

fn mandatory_land_mana_fixture() -> Engine<SyntheticLookup> {
    use baylee_core::ids::{CardIndex, PrintRef};
    use baylee_core::preset::DeckEntry;
    static INSTRUCTION: &[AbilityDef] = &[activated!(
        Cost::FREE,
        &[Effect::ActivateLandsAndTakeMana {
            player: PlayerRel::Chosen
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    )];
    static PRODUCER: &[AbilityDef] = &[activated!(
        Cost::TAP,
        &[Effect::mana(ManaColor::Green, 1)],
        mana_ability = true
    )];
    static CONVERTER: &[AbilityDef] = &[activated!(
        cost!("{1}", TapSelf),
        &[Effect::mana(ManaColor::Red, 2)],
        mana_ability = true
    )];
    let mut preset = synthetic::preset(909_712, &[SOURCE]);
    preset.seats[1].starting_battlefield = [FREE, FILTER, PHASED]
        .map(|id| DeckEntry {
            card: CardIndex::new(id),
            print: PrintRef::new(0),
        })
        .to_vec();
    let lookup = SyntheticLookup::new(vec![
        synthetic::land(SOURCE, "Mandatory mana instruction", INSTRUCTION),
        synthetic::land(FREE, "Cost-free producer", PRODUCER),
        synthetic::land(FILTER, "Paid converter", CONVERTER),
        synthetic::land(PHASED, "Absent producer", PRODUCER),
    ]);
    let mut engine = Engine::new(&preset, lookup).unwrap();
    synthetic::keep_mulligans(&mut engine);
    for _ in 0..20 {
        if matches!(engine.pending(), Pending::Priority { player: ME, .. }) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    engine
}

#[test]
fn mandatory_land_mana_reuses_cost_payment_and_refuses_a_stale_instruction_step() {
    let mut prior_hash = None;
    for _ in 0..2 {
        let mut engine = mandatory_land_mana_fixture();
        let source = synthetic::permanents(&engine, SOURCE)[0];
        let free = synthetic::permanents(&engine, FREE)[0];
        let converter = synthetic::permanents(&engine, FILTER)[0];
        let absent = synthetic::permanents(&engine, PHASED)[0];
        engine
            .state
            .object_mut(absent)
            .unwrap()
            .status
            .insert(crate::object::Status::PHASED_OUT);
        engine.state.players[0].mana_pool.add(ManaColor::Black, 1);
        engine
            .apply(
                ME,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0,
                },
            )
            .unwrap();
        let target = match engine.pending() {
            Pending::ChoosePlayer { .. } => PlayerAction::ChoosePlayer(THEM),
            Pending::ChooseTargets { .. } => PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![THEM],
            },
            pending => panic!("target: {pending:?}"),
        };
        engine.apply(ME, target).unwrap();
        while !matches!(engine.pending(), Pending::ChooseManaAbility { .. }) {
            let pending = engine.pending().clone();
            assert!(synthetic::walk_past(&mut engine, &pending), "{pending:?}");
        }
        let Pending::ChooseManaAbility {
            choice, options, ..
        } = engine.pending().clone()
        else {
            unreachable!()
        };
        assert_eq!(options.len(), 1, "the filter cannot yet pay its cost");
        assert_eq!(options[0].source.object, free);
        let first = PlayerAction::ChooseManaAbility {
            choice,
            source: options[0].source,
            ability_index: options[0].ability_index,
        };
        engine.apply(THEM, first.clone()).unwrap();
        let before = engine.fingerprint();
        assert!(engine.apply(THEM, first).is_err());
        assert_eq!(engine.fingerprint(), before);
        let Pending::ChooseManaAbility {
            choice: second,
            options,
            ..
        } = engine.pending().clone()
        else {
            panic!("filter choice");
        };
        assert_ne!(choice, second);
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].source.object, converter);
        engine
            .apply(
                THEM,
                PlayerAction::ChooseManaAbility {
                    choice: second,
                    source: options[0].source,
                    ability_index: options[0].ability_index,
                },
            )
            .unwrap();
        assert!(engine.state.players[1].mana_pool.is_empty());
        assert_eq!(
            engine.state.players[0]
                .mana_pool
                .available(ManaColor::Black),
            1
        );
        assert_eq!(
            engine.state.players[0].mana_pool.available(ManaColor::Red),
            2
        );
        assert!(synthetic::tapped(&engine, free));
        assert!(synthetic::tapped(&engine, converter));
        assert!(!synthetic::tapped(&engine, absent));
        assert!(engine.state.zones.stack_is_empty());
        if let Some(prior) = prior_hash {
            assert_eq!(engine.snapshot_hash(), prior);
        }
        prior_hash = Some(engine.snapshot_hash());
    }
}
