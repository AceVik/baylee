use super::*;

fn opened() -> Ui {
    let content = baylee_tower_content::standard().unwrap();
    let mut identity = baylee_protocol::execution::Identity::native(
        baylee_protocol::execution::Variant::Tower,
        "test",
        "test",
        22,
    );
    identity.rules = "tower-5".into();
    let cohort = api::Cohort {
        id: Uuid::now_v7(),
        contract: domain::v2::package::Cohort::for_generation(
            identity,
            &content.catalog,
            domain::v2::generation::RUN,
        )
        .unwrap(),
        catalog: content.catalog,
        starters: content.starters,
    };
    let raw = domain::v2::run::Run::begin_for(
        &cohort.catalog,
        cohort.starters[0].clone(),
        &[7; 32],
        domain::v2::generation::RUN,
    )
    .unwrap();
    let run = api::Run::project(Uuid::now_v7(), cohort.id, raw);
    let mut ui = Ui::default();
    let work = ui.model.read(Uuid::now_v7(), Read::Cohorts(None)).unwrap();
    ui.model.receive(
        &work,
        Some(api::Response::Cohorts {
            consent: 1,
            cohorts: vec![cohort],
        }),
        None,
    );
    let work = ui.model.read(Uuid::now_v7(), Read::Run(run.id)).unwrap();
    ui.model
        .receive(&work, Some(api::Response::Run { run: Box::new(run) }), None);
    ui.choices.bind(ui.model.run().unwrap());
    ui
}

#[test]
fn workshop_shows_six_equipment_positions_and_only_exact_unsold_purchase_targets() {
    let ui = opened();
    let run = ui.model.run().unwrap();
    let mut world = World::new();
    let fonts = crate::hud::UiFonts::default();
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
    draw(
        &mut commands,
        root,
        kit,
        &ui,
        run,
        ui.model.publication().unwrap(),
    );
    world.flush();
    assert_eq!(
        world
            .query::<&super::super::super::gestures::Destination>()
            .iter(&world)
            .count(),
        7,
        "six equipment slots and one inventory destination"
    );
    let offers: Vec<_> = world
        .query::<&Press>()
        .iter(&world)
        .filter_map(|press| match press {
            Press::Echoes(Action::Purchase(id)) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(offers.len(), 8);
    assert!(offers.iter().all(|id| {
        run.shop
            .offers
            .iter()
            .any(|offer| offer.id == *id && !offer.sold)
    }));
    assert_eq!(
        world
            .query::<&controls::FocusTooltip>()
            .iter(&world)
            .count(),
        11,
        "eight offers and three supply icons expose detail on focus"
    );
}

#[test]
fn single_purchase_click_creates_one_revision_bound_job_and_keeps_the_confirmed_wallet() {
    let mut ui = opened();
    let run = ui.model.run().unwrap();
    let id = run.shop.offers[0].id;
    let before = run.clone();
    let work = super::super::super::choices::act(&mut ui, Action::Purchase(id)).unwrap();
    assert!(ui.confirmation.is_none());
    assert!(
        matches!(work.request(), Some(api::Request::Command { run, revision, command: Command::Buy { offer, .. }, .. }) if *run == before.id && *revision == before.revision && *offer == id)
    );
    assert_eq!(
        ui.model.run(),
        Some(&before),
        "no speculative gold or item writes"
    );
    assert!(
        super::super::super::choices::act(&mut ui, Action::Purchase(id)).is_none(),
        "busy authority operation cannot duplicate"
    );
}
