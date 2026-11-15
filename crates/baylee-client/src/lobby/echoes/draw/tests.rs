use super::*;
use baylee_core::ids::CardIndex;
use baylee_protocol::execution::{Identity, Variant};
use domain::cards::{Card, Catalog, Collection, Stack};

fn opened() -> Ui {
    let catalog = Catalog {
        cards: (0..=8)
            .map(|id| Card {
                card: CardIndex::new(id),
                price: if id == 0 { 0 } else { 2 },
                basic_land: id == 0,
            })
            .collect(),
    };
    let starter = Collection {
        owned: Vec::new(),
        deck: vec![Stack {
            card: CardIndex::new(0),
            copies: 40,
        }],
    };
    let cohort = api::Cohort {
        id: Uuid::now_v7(),
        contract: domain::v2::package::Cohort::new(
            Identity::native(
                Variant::Tower,
                "ui-test",
                "synthetic",
                u32::from(
                    domain::v2::generation::contract(domain::v2::GENERATOR)
                        .unwrap()
                        .ai_generation,
                ),
            ),
            &catalog,
        )
        .unwrap(),
        catalog,
        starters: [starter.clone(), starter.clone(), starter],
    };
    let raw =
        domain::v2::run::Run::begin(&cohort.catalog, cohort.starters[0].clone(), &[7; 32]).unwrap();
    let raw = raw
        .command(
            0,
            &Command::Buy {
                offer: raw.shop.offers[3].id,
                delivery: domain::v2::shop::Delivery::Slot(Slot::Active(0)),
            },
            &cohort.catalog,
            &[7; 32],
        )
        .unwrap();
    let run = api::Run::project(Uuid::now_v7(), cohort.id, raw);
    let mut ui = Ui::default();
    let read = ui.model.read(Uuid::now_v7(), Read::Cohorts(None)).unwrap();
    ui.model.receive(
        &read,
        Some(api::Response::Cohorts {
            consent: 1,
            cohorts: vec![cohort],
        }),
        None,
    );
    let read = ui.model.read(Uuid::now_v7(), Read::Run(run.id)).unwrap();
    ui.model
        .receive(&read, Some(api::Response::Run { run: Box::new(run) }), None);
    ui.choices.bind(ui.model.run().unwrap());
    ui
}

fn mounted(ui: &Ui) -> World {
    let mut world = World::new();
    let fonts = UiFonts::default();
    let kit = Kit {
        fonts: &fonts,
        german: false,
        m: crate::shellkit::ShellMetrics::of(
            crate::shellkit::Viewport::desktop(1280.0, 720.0),
            crate::shellkit::TextSize::default(),
        ),
    };
    let mut commands = world.commands();
    let root = commands.spawn(Node::default()).id();
    shop::draw(
        &mut commands,
        root,
        kit,
        ui,
        ui.model.run().unwrap(),
        ui.model.publication().unwrap(),
        &Scrolled::default(),
    );
    world.flush();
    world
}

#[test]
fn market_has_one_action_per_offer_destination_and_owned_instance() {
    let mut ui = opened();
    super::super::choices::act(&mut ui, Action::Offer(5));
    let mut world = mounted(&ui);
    let actions: Vec<_> = world
        .query::<&Press>()
        .iter(&world)
        .filter_map(|press| {
            if let Press::Echoes(action) = press {
                Some(*action)
            } else {
                None
            }
        })
        .collect();
    for offer in 1..=8 {
        assert_eq!(
            actions
                .iter()
                .filter(|action| **action == Action::Offer(offer))
                .count(),
            1
        );
    }
    for slot in (0..3).map(Slot::Active).chain((0..6).map(Slot::Reserve)) {
        assert_eq!(
            actions
                .iter()
                .filter(|action| **action == Action::Place(slot))
                .count(),
            1
        );
    }
    assert_eq!(
        actions
            .iter()
            .filter(|action| **action == Action::Select(1))
            .count(),
        1
    );
}

#[test]
fn sold_offers_and_confirming_market_cannot_look_like_live_purchases() {
    let mut ui = opened();
    let mut world = mounted(&ui);
    let disabled = world
        .query::<(&Press, Option<&controls::Disabled>)>()
        .iter(&world)
        .find(|(press, _)| **press == Press::Echoes(Action::Offer(4)))
        .unwrap()
        .1;
    assert!(
        disabled.is_some(),
        "the purchased offer keeps its disabled placeholder"
    );
    super::super::choices::act(&mut ui, Action::Offer(5));
    ui.confirmation = Some(Confirmation::Withdrawal);
    let mut world = mounted(&ui);
    for (press, disabled) in world
        .query::<(&Press, Option<&controls::Disabled>)>()
        .iter(&world)
    {
        if matches!(
            press,
            Press::Echoes(Action::Offer(_) | Action::Select(_) | Action::Place(_))
        ) {
            assert!(
                disabled.is_some(),
                "market hit targets honor the confirmation fence"
            );
        }
    }
}

#[test]
fn inspecting_stock_then_inventory_changes_only_the_single_inspector() {
    let mut ui = opened();
    let before = ui.model.run().unwrap().clone();
    super::super::choices::act(&mut ui, Action::Offer(5));
    assert_eq!(ui.choices.offer, Some(5));
    assert_eq!(ui.choices.item, None);
    super::super::choices::act(&mut ui, Action::Select(1));
    assert_eq!(ui.choices.offer, None);
    assert_eq!(ui.choices.item, Some(1));
    super::super::choices::act(&mut ui, Action::Offer(6));
    assert_eq!(ui.choices.offer, Some(6));
    assert_eq!(ui.choices.item, None);
    assert_eq!(ui.model.run(), Some(&before));
    assert!(!ui.model.busy());
    assert!(ui.confirmation.is_none());
}
