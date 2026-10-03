//! Whole-engine traversal of exact source decisions and delayed references.
use super::synthetic::{self, SyntheticLookup};
use super::*;
use baylee_cards_dsl::{AbilityDef, Amount, Effect, Filter, PlayerRel, TargetSpec};
use baylee_core::ids::DamageSourceRef;

const A: PlayerId = PlayerId::new(0);
const B: PlayerId = PlayerId::new(1);
const SOURCE: u32 = 98_141;
const SHIELD: u32 = 98_142;
static PREVENT: &[AbilityDef] = &[baylee_cards_dsl::activated!(
    baylee_cards_dsl::Cost::FREE,
    &[Effect::PreventNextFromChosenSource {
        sources: &Filter::Any,
        combat_only: false,
        all_but: 0,
        gain_life: false
    }]
)];
static DAMAGE: &[Effect] = &[Effect::DealDamage {
    amount: Amount::Fixed(3),
    target: TargetSpec::Player(PlayerRel::EachOpponent),
}];

fn walk(engine: &mut Engine<SyntheticLookup>, ready: impl Fn(&Engine<SyntheticLookup>) -> bool) {
    for _ in 0..300 {
        if ready(engine) {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(engine, &pending),
            "unexpected {pending:?}"
        );
    }
    panic!("condition not reached: {:?}", engine.pending());
}
fn priority(engine: &mut Engine<SyntheticLookup>, player: PlayerId) {
    walk(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player: asked, .. } if *asked == player),
    );
}
fn choose(engine: &mut Engine<SyntheticLookup>, selected: DamageSourceRef) {
    walk(engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        choice,
        options,
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert!(options.contains(&selected));
    engine
        .apply(
            player,
            PlayerAction::ChooseDamageSource {
                choice,
                source: selected,
            },
        )
        .unwrap();
}
fn activate(engine: &mut Engine<SyntheticLookup>, player: PlayerId, source: ObjectId) {
    priority(engine, player);
    engine
        .apply(
            player,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
}

#[test]
fn source_chosen_as_a_real_permanent_spell_protects_against_its_resolved_permanents_ability() {
    static ACTIVE: &[AbilityDef] = &[baylee_cards_dsl::activated!(
        baylee_cards_dsl::Cost::FREE,
        DAMAGE
    )];
    let definition = Box::leak(Box::new(baylee_cards_dsl::CardDef {
        abilities: ACTIVE,
        faces: Box::leak(Box::new([baylee_cards_dsl::FaceDef {
            castable_from_hand: true,
            mana_cost: baylee_cards_dsl::mana!("{0}"),
            ..synthetic::creature_face("permanent spell source probe", 2, 2, &[])
        }])),
        ..*synthetic::creature(SOURCE, "permanent spell source probe", 2, 2, &[])
    }));
    let shield = synthetic::land(SHIELD, "source shield probe", PREVENT);
    let mut preset = synthetic::preset_both(411, &[], &[SHIELD]);
    preset.seats[0].starting_hand = Some(vec![baylee_core::preset::DeckEntry {
        card: baylee_core::ids::CardIndex::new(SOURCE),
        print: baylee_core::ids::PrintRef::new(0),
    }]);
    let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![definition, shield])).unwrap();
    synthetic::keep_mulligans(&mut engine);
    walk(&mut engine, |e| {
        e.state.turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: A, .. })
    });
    let card = engine.state.zones.list(ZoneLocation::Hand(A))[0];
    engine.apply(A, PlayerAction::CastSpell { card }).unwrap();
    let spell = DamageSourceRef {
        object: card,
        version: engine.state.object(card).unwrap().version,
    };
    let shield = synthetic::permanents(&engine, SHIELD)[0];
    activate(&mut engine, B, shield);
    choose(&mut engine, spell);
    walk(&mut engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(
        engine.state.object(card).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    activate(&mut engine, A, card);
    walk(&mut engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(engine.state.players[1].life, 20);
    assert!(engine.state.shields.is_empty());
}

#[test]
fn actual_cleanup_preserves_delayed_source_and_the_next_turns_trigger_deals_as_that_source() {
    static SCHEDULE: &[AbilityDef] = &[baylee_cards_dsl::activated!(
        baylee_cards_dsl::Cost::FREE,
        &[
            Effect::AtNextEndStep { effects: DAMAGE },
            Effect::ExileSource
        ]
    )];
    let source = synthetic::land(SOURCE, "delayed source probe", SCHEDULE);
    let shield = synthetic::land(SHIELD, "source shield probe", PREVENT);
    let preset = synthetic::preset_both(412, &[SOURCE], &[SHIELD]);
    let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![source, shield])).unwrap();
    synthetic::keep_mulligans(&mut engine);
    walk(&mut engine, |e| {
        e.state.turn.step == Step::End && matches!(e.pending(), Pending::Priority { player: A, .. })
    });
    let id = synthetic::permanents(&engine, SOURCE)[0];
    let was = DamageSourceRef {
        object: id,
        version: engine.state.object(id).unwrap().version,
    };
    activate(&mut engine, A, id);
    walk(&mut engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(
        engine.state.object(id).unwrap().zone,
        crate::zone::Zone::Exile
    );
    let turn = engine.state.turn.number;
    walk(&mut engine, |e| {
        e.state.turn.number > turn
            && e.state.turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: B, .. })
    });
    let shield = synthetic::permanents(&engine, SHIELD)[0];
    activate(&mut engine, B, shield);
    choose(&mut engine, was);
    walk(&mut engine, |e| {
        e.state.turn.step == Step::End && !e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    let trigger = *engine.state.zones.list(ZoneLocation::Stack).last().unwrap();
    assert!(
        engine
            .state
            .object(trigger)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::AbilitySourceVersion(was.version))
    );
    walk(&mut engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(engine.state.players[1].life, 20);
    assert!(engine.state.shields.is_empty());
}

#[test]
fn concession_refreshes_source_identity_and_replay_accepts_only_the_new_choice() {
    let fixture = || {
        let source = synthetic::land(SOURCE, "reference fixture", &[]);
        let shield = synthetic::land(SHIELD, "source shield probe", PREVENT);
        let mut preset = synthetic::preset_both(414, &[SOURCE], &[SHIELD]);
        preset.seats.push(preset.seats[1].clone());
        preset.seats[2].starting_battlefield.clear();
        let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![source, shield])).unwrap();
        while let Pending::Mulligan { player, .. } = engine.pending().clone() {
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        }
        let shield = synthetic::permanents(&engine, SHIELD)[0];
        activate(&mut engine, B, shield);
        walk(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseDamageSource { .. })
        });
        engine
    };
    let mut original = fixture();
    let mut replay = fixture();
    let Pending::ChooseDamageSource {
        choice: old_choice,
        options,
        ..
    } = original.pending().clone()
    else {
        unreachable!()
    };
    let selected = options[0];
    for engine in [&mut original, &mut replay] {
        engine
            .apply(PlayerId::new(2), PlayerAction::Concede)
            .unwrap();
        let Pending::ChooseDamageSource {
            choice, options, ..
        } = engine.pending().clone()
        else {
            panic!("source decision lost")
        };
        assert_ne!(choice, old_choice);
        assert!(options.contains(&selected));
        let hash = engine.snapshot_hash();
        assert!(
            engine
                .apply(
                    B,
                    PlayerAction::ChooseDamageSource {
                        choice: old_choice,
                        source: selected
                    }
                )
                .is_err()
        );
        assert_eq!(engine.snapshot_hash(), hash);
        engine
            .apply(
                B,
                PlayerAction::ChooseDamageSource {
                    choice,
                    source: selected,
                },
            )
            .unwrap();
    }
    assert_eq!(original.snapshot_hash(), replay.snapshot_hash());
}

#[test]
fn concession_removing_every_matching_source_skips_only_that_instruction() {
    static WHITE: Filter = Filter::HasColor(baylee_core::color::ColorSet::of(
        baylee_core::color::Color::White,
    ));
    static FILTERED: &[AbilityDef] = &[baylee_cards_dsl::activated!(
        baylee_cards_dsl::Cost::FREE,
        &[
            Effect::PreventNextFromChosenSource {
                sources: &WHITE,
                combat_only: false,
                all_but: 0,
                gain_life: false
            },
            Effect::GainLife {
                amount: Amount::Fixed(1)
            },
        ]
    )];
    let source = synthetic::land(SOURCE, "reference fixture", &[]);
    let shield = synthetic::land(SHIELD, "source shield probe", FILTERED);
    let mut preset = synthetic::preset_both(415, &[SOURCE], &[SHIELD]);
    preset.seats.push(preset.seats[1].clone());
    preset.seats[2].starting_battlefield.clear();
    let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![source, shield])).unwrap();
    while let Pending::Mulligan { player, .. } = engine.pending().clone() {
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    let source = synthetic::permanents(&engine, SOURCE)[0];
    engine.state.object_mut(source).unwrap().base_mut().colors =
        baylee_core::color::ColorSet::of(baylee_core::color::Color::White);
    engine.state.invalidate_projections();
    engine.state.refresh_characteristics();
    let shield = synthetic::permanents(&engine, SHIELD)[0];
    activate(&mut engine, B, shield);
    walk(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    engine.apply(A, PlayerAction::Concede).unwrap();
    assert!(!matches!(
        engine.pending(),
        Pending::ChooseDamageSource { .. }
    ));
    assert_eq!(engine.state.players[1].life, 21);
    assert!(engine.state.shields.is_empty());
}
