use super::*;
use baylee_client_core::test_support::{ViewBuilder, printed};
use baylee_core::{mana::ManaCost, types::TypeSet};

#[test]
fn a_tap_self_cost_cannot_also_spend_the_sources_mana() {
    let card = baylee_cards::decks::by_name("Kessig Wolf Run").unwrap();
    let mut object = printed(0, 0, "Kessig Wolf Run", 0);
    object.card.as_mut().unwrap().index = card;
    object.rules.as_mut().unwrap().card = card;
    object.types = TypeSet::LAND;
    let source = object.id;
    let mut view = ViewBuilder::new(2).with_battlefield(0, [object]).build();
    let legal = LegalActions {
        abilities: vec![(source, 0)],
        ..LegalActions::default()
    };
    let mut definition = baylee_cards::by_index(card).unwrap().abilities[1];
    let AbilityDef::Activated { cost, x_value, .. } = &mut definition else {
        panic!("activated fixture")
    };
    cost.mana = ManaCost::parse("{X}");
    *x_value = ActivationX::AtLeast(1);
    assert!(!reachable(&view, &legal, source, &definition));

    // A source is usable when the announced ability does not tap it.
    let AbilityDef::Activated { cost, .. } = &mut definition else {
        unreachable!()
    };
    cost.parts = &[];
    assert!(reachable(&view, &legal, source, &definition));

    let AbilityDef::Activated { cost, x_value, .. } = &mut definition else {
        unreachable!()
    };
    cost.parts = &[CostPart::TapSelf];
    *x_value = ActivationX::AtLeast(2);
    view.seats[0].mana_pool.colorless = 2;
    assert!(reachable(&view, &legal, source, &definition));
}
