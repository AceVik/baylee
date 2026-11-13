use super::*;
use baylee_client_core::{
    Interaction,
    test_support::{ViewBuilder, printed},
};
use baylee_core::{
    generated::subtypes::land,
    ids::PlayerId,
    mana::ManaCost,
    types::{SubtypeSet, TypeSet},
};
use baylee_engine::choice::{CastModeDesc, CastModeKind, LegalActions, Pending, PlayerAction};
use baylee_view::PublicObject;

fn library_card(slot: u32, name: &str) -> PublicObject {
    let index = baylee_cards::decks::by_name(name).expect("a registered library card");
    let mut object = printed(slot, 0, name, 0);
    object.card.as_mut().expect("a card identity").index = index;
    object.rules = object.card.map(baylee_view::RulesFace::from);
    object.types = baylee_cards::by_index(index)
        .expect("a card definition")
        .faces[0]
        .types;
    object
}

fn life_kind(life: u32, face: usize, kicked: bool) -> CastModeKind {
    CastModeKind::LibraryTopLife {
        life,
        face,
        modes: 0,
        kicked,
    }
}

fn life_way(index: u32, life: u32, face: usize, kicked: bool, cost: ManaCost) -> CastModeDesc {
    CastModeDesc {
        index,
        kind: life_kind(life, face, kicked),
        cost,
    }
}

fn priority(player: PlayerId, legal: LegalActions) -> Pending {
    Pending::Priority {
        player,
        legal: Box::new(legal),
    }
}

fn island(slot: u32) -> PublicObject {
    let mut island = library_card(slot, "Island");
    island.types = TypeSet::LAND;
    island.subtypes = SubtypeSet::from_slice(&[land::ISLAND]);
    island.power = None;
    island.toughness = None;
    island
}

#[test]
fn a_revealed_library_land_click_uses_the_authoritative_offer() {
    let mut view = ViewBuilder::new(2).build();
    let top = printed(100, 0, "Forest", 1);
    let id = top.id;
    view.library_tops.push(top);
    let pending = |lands| Pending::Priority {
        player: view.seat,
        legal: Box::new(LegalActions {
            lands,
            ..Default::default()
        }),
    };
    let mut duel = Duel {
        interaction: Some(Interaction::new(pending(vec![id]), view.seat)),
        view: Some(view.clone()),
        ..Duel::default()
    };
    activate_card(&mut duel, id);
    assert!(matches!(duel.outbox.as_slice(), [PlayerAction::PlayLand { card }] if *card == id));
    duel.outbox.clear();
    duel.interaction = Some(Interaction::new(pending(vec![]), view.seat));
    activate_card(&mut duel, id);
    assert!(duel.outbox.is_empty(), "visible does not imply playable");
}

#[test]
fn one_library_life_route_still_opens_a_chooser_and_is_remembered() {
    let mut view = ViewBuilder::new(2).build();
    let top = library_card(100, "Dark Ritual");
    let id = top.id;
    view.library_tops.push(top);
    let kind = life_kind(1, 0, false);
    let legal = LegalActions {
        castable: vec![id],
        cast_ways: vec![(id, life_way(0, 1, 0, false, ManaCost::ZERO))],
        ..Default::default()
    };
    let mut duel = Duel {
        interaction: Some(Interaction::new(priority(view.seat, legal), view.seat)),
        view: Some(view),
        ..Duel::default()
    };

    activate_card(&mut duel, id);
    let menu = duel.cast_menu.as_ref().expect("the exact route is chosen");
    assert_eq!(menu.modes.len(), 1);
    assert_eq!(menu.modes[0].kind, kind);
    assert!(menu.modes[0].plan.steps.is_empty());
    take_cast_row(&mut duel, 0);
    assert_eq!(duel.cast_answer, Some((id, kind)));
}

#[test]
fn a_zero_mana_adventure_route_survives_cast_and_mode_answer() {
    let mut view = ViewBuilder::new(2).build();
    let top = library_card(100, "Brazen Borrower");
    let id = top.id;
    view.library_tops.push(top);
    let options = vec![
        life_way(0, 3, 0, false, ManaCost::ZERO),
        life_way(1, 2, 1, false, ManaCost::ZERO),
    ];
    let legal = LegalActions {
        castable: vec![id],
        cast_ways: options.iter().map(|way| (id, way.clone())).collect(),
        ..Default::default()
    };
    let mut duel = Duel {
        interaction: Some(Interaction::new(priority(view.seat, legal), view.seat)),
        view: Some(view.clone()),
        ..Duel::default()
    };

    activate_card(&mut duel, id);
    let menu = duel.cast_menu.as_ref().expect("both faces are offered");
    assert_eq!(menu.modes.len(), 2, "no normal hand cast is invented");
    assert!(menu.modes.iter().all(|mode| mode.plan.steps.is_empty()));
    let pick = menu
        .modes
        .iter()
        .position(|mode| mode.kind == options[1].kind)
        .expect("Petty Theft is its own route");
    take_cast_row(&mut duel, pick);
    fire_armed(&mut duel);
    crate::advance_mana_run(&mut duel);
    assert_eq!(
        duel.take_outbox(),
        vec![PlayerAction::CastSpell { card: id }]
    );

    duel.interaction = Some(Interaction::new(
        Pending::ChooseCastMode {
            player: view.seat,
            object: id,
            options,
        },
        view.seat,
    ));
    crate::take_the_chosen_cast_mode(&mut duel);
    assert_eq!(duel.take_outbox(), vec![PlayerAction::ChooseMode(1)]);
    assert_eq!(duel.cast_answer, None);
}

#[test]
fn a_library_kicker_plans_five_taps_and_answers_the_kicked_route() {
    let islands: Vec<_> = (200..205).map(island).collect();
    let sources: Vec<_> = islands.iter().map(|island| island.id).collect();
    let mut view = ViewBuilder::new(2).with_battlefield(0, islands).build();
    let top = library_card(100, "Rite of Replication");
    let id = top.id;
    view.library_tops.push(top);
    let options = vec![
        life_way(0, 4, 0, false, ManaCost::ZERO),
        life_way(1, 4, 0, true, ManaCost::parse("{5}")),
    ];
    let legal = LegalActions {
        can_pass: true,
        castable: vec![id],
        mana_abilities: sources.clone(),
        cast_ways: options.iter().map(|way| (id, way.clone())).collect(),
        ..Default::default()
    };
    let mut duel = Duel {
        interaction: Some(Interaction::new(
            priority(view.seat, legal.clone()),
            view.seat,
        )),
        view: Some(view.clone()),
        ..Duel::default()
    };

    activate_card(&mut duel, id);
    let menu = duel.cast_menu.as_ref().expect("kicker is a separate route");
    let pick = menu
        .modes
        .iter()
        .position(|mode| mode.kind == options[1].kind)
        .expect("the kicked route is offered");
    assert_eq!(menu.modes[pick].plan.steps.len(), 5);
    assert!(menu.modes[1 - pick].plan.steps.is_empty());
    take_cast_row(&mut duel, pick);
    fire_armed(&mut duel);

    let mut tapped = Vec::new();
    for _ in 0..5 {
        crate::advance_mana_run(&mut duel);
        let actions = duel.take_outbox();
        assert_eq!(actions.len(), 1);
        let PlayerAction::ActivateManaAbility { source } = actions[0] else {
            panic!("the run taps only the remaining kicker mana: {actions:?}");
        };
        tapped.push(source);
    }
    tapped.sort();
    let mut expected = sources;
    expected.sort();
    assert_eq!(tapped, expected);
    crate::advance_mana_run(&mut duel);
    assert_eq!(
        duel.take_outbox(),
        vec![PlayerAction::CastSpell { card: id }]
    );

    duel.interaction = Some(Interaction::new(
        Pending::ChooseCastMode {
            player: view.seat,
            object: id,
            options,
        },
        view.seat,
    ));
    crate::take_the_chosen_cast_mode(&mut duel);
    assert_eq!(duel.take_outbox(), vec![PlayerAction::ChooseMode(1)]);
}

#[test]
fn a_library_mdfc_keeps_its_land_drop_beside_its_life_route() {
    let mut view = ViewBuilder::new(2).build();
    let top = library_card(100, "Boggart Trawler");
    let id = top.id;
    view.library_tops.push(top);
    let kind = life_kind(3, 0, false);
    let legal = LegalActions {
        lands: vec![id],
        castable: vec![id],
        cast_ways: vec![(id, life_way(0, 3, 0, false, ManaCost::ZERO))],
        ..Default::default()
    };

    let modes = crate::castmodes::reachable_modes(&view, &legal, id);
    assert_eq!(
        modes.iter().map(|mode| mode.kind).collect::<Vec<_>>(),
        vec![kind, CastModeKind::PlayLandFace(1)],
        "only the engine's spell route and the actual land face are offered"
    );
    assert!(modes.iter().all(|mode| mode.plan.steps.is_empty()));
}

#[test]
fn a_payable_library_life_route_accepts_an_empty_mana_plan() {
    let mut view = ViewBuilder::new(2).build();
    let top = library_card(100, "Dark Ritual");
    let id = top.id;
    view.library_tops.push(top);
    let legal = LegalActions {
        payable: vec![id],
        cast_ways: vec![(id, life_way(0, 1, 0, false, ManaCost::ZERO))],
        ..Default::default()
    };
    let duel = Duel {
        interaction: Some(Interaction::new(priority(view.seat, legal), view.seat)),
        view: Some(view),
        ..Duel::default()
    };

    assert!(crate::reachable(&duel).contains(&id));
    let plan = crate::mana_for(&duel, id).expect("zero remaining mana is a valid plan");
    assert!(plan.steps.is_empty());
}
