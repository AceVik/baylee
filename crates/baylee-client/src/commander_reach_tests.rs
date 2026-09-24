use super::*;
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::mana::ManaColor;
use baylee_engine::choice::GRANTED_ABILITY;

/// `n` lands that each make one mana of any colour, and the engine
/// offering every one of them.
///
/// Any-colour sources on purpose: what is under test is whether the
/// command zone is *looked at*, and a test that also had to get the
/// colours right would fail for two reasons and say one.
pub(crate) fn table_with(lands: usize, commander_casts: u32) -> Duel {
    let mut objects = Vec::new();
    for slot in 0..lands {
        let mut land = token(100 + slot as u32, 0, "Wastes", 0, 0);
        land.types = baylee_core::types::TypeSet::LAND;
        land.power = None;
        land.toughness = None;
        land.granted_mana = Some(baylee_view::GrantedMana {
            slot: 0,
            colors: vec![
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green,
            ],
            amount: 1,
        });
        objects.push(land);
    }
    let ids: Vec<_> = objects.iter().map(|o| o.id).collect();
    let mut commander = crate::registry_printed(7, 0, "Katara, the Fearless");
    commander.commander = true;
    let mut view = ViewBuilder::new(2)
        .with_battlefield(0, objects)
        .with_command(0, vec![commander.clone()])
        .with_commanders(0, &[&commander])
        .build();
    view.seats[0].commanders[0].casts = commander_casts;
    let legal = baylee_engine::choice::LegalActions {
        can_pass: true,
        abilities: ids.iter().map(|id| (*id, GRANTED_ABILITY)).collect(),
        mana_abilities: ids,
        ..baylee_engine::choice::LegalActions::default()
    };
    Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            baylee_engine::choice::Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(legal),
            },
            PlayerId::new(0),
        )),
        view: Some(view),
        ..Default::default()
    }
}

/// The commander is a card this client will tap lands for.
///
/// It was not, and that is the whole of "I cannot play my commander":
/// `reachable` read `view.hand` and nothing else, so the one card a
/// commander deck is built around was never in it. With the mana not
/// already floating the engine has not offered the card either, so a
/// click found no action, no ability and no selection — and fell through
/// to the last branch of `activate_card`, which opens the zone browser.
/// Pressing the commander opened a panel about the commander.
#[test]
fn a_commander_in_the_command_zone_is_reachable() {
    let duel = table_with(3, 0);
    let reach = reachable(&duel);
    let commander = duel.view.as_ref().unwrap().seats[0].commanders[0].object;
    assert!(
        reach.contains(&commander),
        "three lands pay {{G}}{{W}}{{U}} and the commander was not offered: {reach:?}"
    );
    assert!(
        mana_for(&duel, commander).is_some(),
        "and the plan that click would run has to exist too, or the card \
         lights up and then does nothing"
    );
}

/// And CR 903.8's tax is part of what it costs.
///
/// A commander cast once already needs `{2}` more. Three lands paid for
/// it the first time and must not the second, or the client offers a
/// plan the engine refuses — which is the failure mode the mana planner
/// exists to avoid, not one to introduce at a new door.
#[test]
fn the_commander_tax_is_part_of_what_the_client_plans_for() {
    let once = table_with(3, 1);
    let commander = once.view.as_ref().unwrap().seats[0].commanders[0].object;
    assert!(
        !reachable(&once).contains(&commander),
        "three lands do not pay {{G}}{{W}}{{U}} plus the {{2}} tax"
    );
    assert!(
        reachable(&table_with(5, 1)).contains(&commander),
        "and five do"
    );
}
