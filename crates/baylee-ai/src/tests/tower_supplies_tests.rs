use super::*;

fn potion(kind: baylee_core::preset::TowerSupply, charges: u16) -> PublicObject {
    let definition = baylee_cards::tokens::tower::definition(kind);
    let mut p = permanent(obj(1), PlayerId::new(0), 0);
    p.name = definition.name.into();
    p.types = TypeSet::EMPTY;
    p.power = None;
    p.toughness = None;
    p.token = Some(baylee_cards::tokens::token_id(definition));
    p.counters = vec![CounterEntry {
        kind: CounterKind::Charge,
        count: charges,
    }];
    p
}

#[test]
fn tower_expert_reserves_healing_at_full_life_and_uses_it_when_hurt() {
    let mut v = view(0, &[20, 20], vec![]);
    v.command[0].push(potion(baylee_core::preset::TowerSupply::HealingPotion, 1));
    let pending = offering(vec![(obj(1), 0)]);
    let expert = HeuristicAgent::new(AIProfile::named("expert").unwrap());
    assert_eq!(expert.act(&v, &pending), PlayerAction::PassPriority);
    v.seats[0].life = 5;
    assert_eq!(
        expert.act(&v, &pending),
        PlayerAction::ActivateAbility {
            source: obj(1),
            ability_index: 0
        }
    );
}

#[test]
fn tower_mana_charges_are_known_sources_and_are_spent_for_a_reachable_spell() {
    let mut v = view(0, &[20, 20], vec![]);
    v.phase = baylee_view::Phase::FirstMain;
    v.command[0].push(potion(baylee_core::preset::TowerSupply::ManaPotion, 2));
    v.hand = vec![hand_card(2, "Brainstorm")];
    let legal = baylee_engine::choice::LegalActions {
        can_pass: true,
        payable: vec![obj(2)],
        abilities: vec![(obj(1), 0)],
        ..Default::default()
    };
    let mut sources = crate::mana_sources(&v, &legal);
    crate::spend::repeat_counter_sources(&v, &mut sources);
    assert_eq!(
        sources.len(),
        2,
        "two charges buy two mana, never an unlimited source"
    );
    assert!(
        sources
            .iter()
            .all(|s| s.priced && s.colors.len() == 5 && s.amount == 1)
    );
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(legal),
    };
    let expert = HeuristicAgent::new(AIProfile::named("expert").unwrap());
    assert_eq!(
        expert.act(&v, &pending),
        PlayerAction::ActivateAbility {
            source: obj(1),
            ability_index: 0
        }
    );
    v.hand = vec![hand_card(2, "Darksteel Forge")];
    assert_eq!(
        expert.act(&v, &pending),
        PlayerAction::PassPriority,
        "do not waste charges on an unaffordable spell"
    );
}
