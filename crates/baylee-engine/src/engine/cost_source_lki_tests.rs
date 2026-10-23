//! Resolution reads the source incarnation whose activation paid the cost.
use super::synthetic::{self, SyntheticLookup};
use super::*;
use baylee_cards_dsl::{AbilityDef, Amount, CounterKind, Effect, PlayerRel, TargetSpec};
use baylee_core::mana::ManaColor;

const YOU: PlayerId = PlayerId::new(0);
const SOURCE: u32 = 98_160;
const GROW: AbilityDef = baylee_cards_dsl::activated!(
    baylee_cards_dsl::Cost::FREE,
    &[Effect::AddCounter {
        kind: CounterKind::P1P1,
        amount: Amount::Fixed(3),
    }]
);
const DAMAGE: &[Effect] = &[Effect::DealDamage {
    amount: Amount::SourcePower,
    target: TargetSpec::Player(PlayerRel::EachOpponent),
}];

fn walk(engine: &mut Engine<SyntheticLookup>, ready: impl Fn(&Engine<SyntheticLookup>) -> bool) {
    for _ in 0..100 {
        if ready(engine) {
            return;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(engine, &pending), "{pending:?}");
    }
    panic!("condition not reached: {:?}", engine.pending());
}

fn settle(engine: &mut Engine<SyntheticLookup>) {
    walk(engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
            && matches!(e.pending(), Pending::Priority { player: YOU, .. })
    });
}

fn setup(abilities: &'static [AbilityDef]) -> (Engine<SyntheticLookup>, ObjectId) {
    let card = Box::leak(Box::new(baylee_cards_dsl::CardDef {
        abilities,
        ..*synthetic::creature(SOURCE, "Cost source probe", 2, 2, &[])
    }));
    let mut engine = Engine::new(
        &synthetic::preset(98_160, &[SOURCE]),
        SyntheticLookup::new(vec![card]),
    )
    .unwrap();
    synthetic::keep_mulligans(&mut engine);
    walk(&mut engine, |e| {
        e.state.turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: YOU, .. })
    });
    let source = synthetic::permanents(&engine, SOURCE)[0];
    activate(&mut engine, source, 0);
    settle(&mut engine);
    assert_eq!(
        engine.state.object(source).unwrap().characteristics().power,
        Some(5)
    );
    (engine, source)
}

fn activate(engine: &mut Engine<SyntheticLookup>, source: ObjectId, ability_index: u32) {
    engine
        .apply(
            YOU,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
}

#[test]
fn sacrificed_source_power_survives_payment_and_a_later_incarnation() {
    static ABILITIES: &[AbilityDef] = &[
        GROW,
        baylee_cards_dsl::activated!(baylee_cards_dsl::cost!(SacrificeSelf), DAMAGE),
    ];
    for returns in [false, true] {
        let (mut engine, source) = setup(ABILITIES);
        activate(&mut engine, source, 1);
        assert_eq!(engine.state.object(source).unwrap().zone, Zone::Graveyard);
        if returns {
            engine
                .state
                .move_object(
                    source,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            engine.state.refresh_characteristics();
            assert_eq!(
                engine.state.object(source).unwrap().characteristics().power,
                Some(2)
            );
        }
        settle(&mut engine);
        assert_eq!(engine.state.players[1].life, 15, "returns: {returns}");
    }
}

#[test]
fn surviving_source_power_remains_live_until_resolution() {
    static ABILITIES: &[AbilityDef] = &[
        GROW,
        baylee_cards_dsl::activated!(baylee_cards_dsl::Cost::FREE, DAMAGE),
    ];
    let (mut engine, source) = setup(ABILITIES);
    activate(&mut engine, source, 1);
    activate(&mut engine, source, 0);
    settle(&mut engine);
    assert_eq!(engine.state.players[1].life, 12);
}

#[test]
fn sacrificing_mana_ability_uses_counters_remaining_after_costs_and_resumes_color_choice() {
    static ABILITIES: &[AbilityDef] = &[
        GROW,
        baylee_cards_dsl::mana_ability!(
            baylee_cards_dsl::cost!(
                RemoveCounterSelf {
                    kind: CounterKind::P1P1,
                    n: 1
                },
                SacrificeSelf
            ),
            &[Effect::mana_choice_dynamic(
                &[ManaColor::Blue, ManaColor::Green],
                Amount::CountersOnSource(CounterKind::P1P1),
            )]
        ),
    ];
    let (mut engine, source) = setup(ABILITIES);
    activate(&mut engine, source, 1);
    assert_eq!(engine.state.object(source).unwrap().zone, Zone::Graveyard);
    assert!(engine.state.zones.list(ZoneLocation::Stack).is_empty());
    assert!(matches!(
        engine.pending(),
        Pending::ChooseColor { player: YOU, .. }
    ));
    engine
        .apply(YOU, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(
        engine.state.players[0].mana_pool.available(ManaColor::Blue),
        2
    );
}

#[test]
fn sacrificing_mana_ability_reads_source_power_without_a_stack_object() {
    static ABILITIES: &[AbilityDef] = &[
        GROW,
        baylee_cards_dsl::mana_ability!(
            baylee_cards_dsl::cost!(SacrificeSelf),
            &[Effect::mana_dynamic(ManaColor::Green, Amount::SourcePower)]
        ),
    ];
    let (mut engine, source) = setup(ABILITIES);
    activate(&mut engine, source, 1);
    assert_eq!(engine.state.object(source).unwrap().zone, Zone::Graveyard);
    assert!(engine.state.zones.list(ZoneLocation::Stack).is_empty());
    assert_eq!(
        engine.state.players[0]
            .mana_pool
            .available(ManaColor::Green),
        5
    );
}
