//! Entry choices invalidate characteristics before the next offered action.

use super::*;
use baylee_cards_dsl::{EnterModifier, Filter, Modifier, PtCount};

static COUNT: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::This,
    Modifier::CharacteristicPT {
        count: PtCount::ChosenPlayerControls(&Filter::CREATURE),
        toughness_plus: 1,
    }
)];

#[test]
fn choosing_an_opponent_at_entry_refreshes_the_characteristic_count_immediately() {
    let mut engine = bench(
        7955,
        vec![
            card(
                7955,
                FaceDef {
                    enter_modifiers: &[EnterModifier::ChooseOpponent],
                    ..creature_face("Chosen count", "{0}", 0, 1)
                },
                KeywordSet::EMPTY,
                COUNT,
            ),
            card(
                7956,
                creature_face("Counted creature", "{0}", 1, 2),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::default().holding(&[7955]), Seat::with(&[7956, 7956])],
    );
    to_main(&mut engine, me());
    let subject = the(&engine, ZoneLocation::Hand(me()), 7955);
    engine
        .apply(me(), PlayerAction::CastSpell { card: subject })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine
        .apply(me(), PlayerAction::ChoosePlayer(them()))
        .unwrap();
    settle(&mut engine, me());
    let characteristics = engine.state().object(subject).unwrap().characteristics();
    assert_eq!(
        (characteristics.power, characteristics.toughness),
        (Some(2), Some(3))
    );
}
