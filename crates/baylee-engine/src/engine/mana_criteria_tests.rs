//! CR 605.1a execution does not trust an obsolete authoring flag.
use super::synthetic::{self, SyntheticLookup};
use super::*;
use baylee_cards_dsl::prelude::*;

const ME: PlayerId = PlayerId::new(0);
const SOURCE: u32 = 995_130;
const OUTPUT: &[Effect] = &[Effect::mana(ManaColor::Blue, 1), Effect::draw(1)];
const PAYMENT: AbilityDef = activated!(
    Cost::FREE,
    &[Effect::PayManaToPreventDamage {
        player: PlayerRel::You,
        amount: Amount::Fixed(0),
    }]
);
const PRINTED: &[AbilityDef] = &[mana_ability!(Cost::TAP, OUTPUT), PAYMENT];
const GRANTED: &[AbilityDef] = &[
    static_ability!(
        Filter::This,
        Modifier::GrantActivated {
            cost: Cost::TAP,
            effects: OUTPUT,
            mana_ability: true,
        }
    ),
    PAYMENT,
];

fn fixture(granted: bool) -> (Engine<SyntheticLookup>, ObjectId, u32) {
    let mut engine = Engine::new(
        &synthetic::preset(909_713, &[SOURCE]),
        SyntheticLookup::new(vec![synthetic::land(
            SOURCE,
            "Library mana probe",
            if granted { GRANTED } else { PRINTED },
        )]),
    )
    .unwrap();
    synthetic::keep_mulligans(&mut engine);
    for _ in 0..30 {
        if matches!(engine.pending(), Pending::Priority { player: ME, .. }) {
            let source = synthetic::permanents(&engine, SOURCE)[0];
            return (
                engine,
                source,
                if granted {
                    crate::choice::granted_ability(0)
                } else {
                    0
                },
            );
        }
        let question = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &question));
    }
    panic!("no priority");
}

#[test]
fn printed_and_granted_library_movers_use_the_stack_despite_a_stale_mana_flag() {
    for granted in [false, true] {
        let (mut engine, source, ability_index) = fixture(granted);
        let before_hand = engine.state.zones.list(ZoneLocation::Hand(ME)).len();
        engine
            .apply(
                ME,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                },
            )
            .unwrap();
        assert_eq!(engine.state.zones.list(ZoneLocation::Stack).len(), 1);
        assert_eq!(engine.state.players[0].mana_pool.total(), 0);
        assert_eq!(
            engine.state.zones.list(ZoneLocation::Hand(ME)).len(),
            before_hand
        );
        for _ in 0..8 {
            if engine.state.zones.stack_is_empty() {
                break;
            }
            let question = engine.pending().clone();
            assert!(synthetic::walk_past(&mut engine, &question));
        }
        assert_eq!(
            engine.state.players[0].mana_pool.available(ManaColor::Blue),
            1
        );
        assert_eq!(
            engine.state.zones.list(ZoneLocation::Hand(ME)).len(),
            before_hand + 1
        );
    }
}

#[test]
fn printed_and_granted_library_movers_are_not_payment_activations() {
    for granted in [false, true] {
        let (mut engine, source, ability_index) = fixture(granted);
        engine
            .apply(
                ME,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 1,
                },
            )
            .unwrap();
        for _ in 0..8 {
            if engine.payment_window().is_some() {
                break;
            }
            let question = engine.pending().clone();
            assert!(synthetic::walk_past(&mut engine, &question));
        }
        assert!(engine.payment_window().is_some());
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("mana opportunity");
        };
        assert!(!legal.abilities.contains(&(source, ability_index)));
        assert!(!legal.mana_abilities.contains(&source));
        let before = engine.fingerprint();
        assert!(
            engine
                .apply(
                    ME,
                    PlayerAction::ActivateAbility {
                        source,
                        ability_index
                    }
                )
                .is_err()
        );
        assert_eq!(engine.fingerprint(), before);
        assert!(!synthetic::tapped(&engine, source));
    }
}
