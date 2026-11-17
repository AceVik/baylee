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
        offered: vec![0, 1, 2, 3, 4],
    };
    let raw = domain::v2::run::Run::begin_for(
        &cohort.catalog,
        cohort.starters[0].clone(),
        &[7; 32],
        domain::v2::generation::RUN,
    )
    .unwrap();
    let run = api::Run::project(Uuid::now_v7(), cohort.id, raw);
    let mut ui = Ui {
        opened: true,
        ..Default::default()
    };
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

fn test_kit(fonts: &crate::hud::UiFonts, german: bool) -> Kit<'_> {
    Kit {
        fonts,
        german,
        m: crate::shellkit::ShellMetrics::of(
            crate::shellkit::Viewport::desktop(1280.0, 720.0),
            crate::shellkit::TextSize::default(),
        ),
    }
}

#[test]
fn every_tower_ui_glyph_exists_in_shipped_fontawesome() {
    use swash::FontRef;
    let bytes = include_bytes!("../../../../../assets/fonts/fa-solid-900.ttf");
    let font = FontRef::from_index(bytes, 0).expect("valid fa-solid-900.ttf");
    let charmap = font.charmap();
    for glyph in words::TOWER_UI_GLYPHS {
        assert_ne!(
            charmap.map(glyph),
            0,
            "Font Awesome glyph U+{:04X} ({glyph:?}) missing from shipped fa-solid-900.ttf",
            glyph as u32
        );
    }
}

#[test]
fn workshop_shows_six_equipment_positions_and_only_exact_unsold_purchase_targets() {
    let ui = opened();
    let run = ui.model.run().unwrap();
    let mut world = World::new();
    let fonts = crate::hud::UiFonts::default();
    let kit = test_kit(&fonts, false);
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
fn inventory_omits_redundant_text_and_renders_five_icon_sorts_plus_groups_without_collapse() {
    for german in [true, false] {
        let mut ui = opened();
        let lang = if german { Lang::De } else { Lang::En };
        for grouped in [false, true] {
            ui.bag.grouped = grouped;
            let run = ui.model.run().unwrap();
            let mut world = World::new();
            let fonts = crate::hud::UiFonts::default();
            let kit = test_kit(&fonts, german);
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

            let texts: Vec<String> = world
                .query::<&Text>()
                .iter(&world)
                .map(|t| t.0.clone())
                .collect();
            assert!(
                !texts
                    .iter()
                    .any(|t| t == Phrase::EchoesInventory.text(lang)),
                "bag header label removed in {lang:?}"
            );
            assert!(
                !texts
                    .iter()
                    .any(|t| t == Phrase::EchoesRunBagEmpty.text(lang)),
                "empty bag helper text removed in {lang:?}"
            );

            for order in workshop::Order::ALL {
                let btn = world
                    .query::<(Entity, &Press, Option<&controls::Tooltip>)>()
                    .iter(&world)
                    .find(|(_, press, _)| **press == Press::Echoes(Action::InventoryOrder(order)))
                    .unwrap_or_else(|| panic!("missing icon sort button {order:?}"));
                assert!(btn.2.is_some(), "icon sort {order:?} carries hover tooltip");
            }
            let group_btn = world
                .query::<(&Press, Option<&controls::Tooltip>)>()
                .iter(&world)
                .find(|(press, _)| **press == Press::Echoes(Action::InventoryGroups(!grouped)))
                .expect("groups toggle button");
            assert!(group_btn.1.is_some());

            let diamond_slots = world
                .query::<&Text>()
                .iter(&world)
                .filter(|t| t.0 == glyph::DIAMOND.to_string())
                .count();
            assert!(
                diamond_slots >= workshop::PAGE_SIZE,
                "empty bag retains all 18 placeholder slots even when grouped={grouped}"
            );
        }
    }
}

#[test]
fn frozen_shelf_shows_frost_badge_and_spare_cards_sell_cleanly() {
    let mut ui = opened();
    let cohort = ui.model.publication().unwrap().clone();
    let mut run = ui.model.run().unwrap().clone();
    run.shop.frozen = true;
    let spare_card = cohort
        .catalog
        .cards
        .iter()
        .find(|entry| {
            !entry.basic_land
                && run.cards.copies(entry.card) == 0
                && run
                    .cards
                    .formation
                    .as_ref()
                    .is_none_or(|f| f.commander != entry.card)
        })
        .expect("an unowned nonbasic card in catalog")
        .card;
    run.cards.owned.push(domain::cards::Stack {
        card: spare_card,
        copies: 1,
    });
    run.revision += 1;
    let read = ui.model.read(Uuid::now_v7(), Read::Run(run.id)).unwrap();
    ui.model.receive(
        &read,
        Some(api::Response::Run {
            run: Box::new(run.clone()),
        }),
        None,
    );
    ui.choices.bind(ui.model.run().unwrap());
    ui.spare_cards_open = true;

    let mut world = World::new();
    let fonts = crate::hud::UiFonts::default();
    let kit = test_kit(&fonts, true);
    let mut commands = world.commands();
    let root = commands.spawn(Node::default()).id();
    draw(
        &mut commands,
        root,
        kit,
        &ui,
        ui.model.run().unwrap(),
        &cohort,
    );
    world.flush();

    let texts: Vec<String> = world
        .query::<&Text>()
        .iter(&world)
        .map(|t| t.0.clone())
        .collect();
    assert!(
        texts
            .iter()
            .any(|t| t == Phrase::EchoesShelfFrozen.text(Lang::De)),
        "frozen shelf displays unmistakable frost badge"
    );
    let frozen_portraits = world
        .query::<&Emphasis>()
        .iter(&world)
        .filter(|&&e| e == Emphasis::Frozen)
        .count();
    assert_eq!(
        frozen_portraits, 16,
        "every unsold offer's shelf face and detail overlay portrait wear Emphasis::Frozen (8 × 2)"
    );

    let sell_buttons: Vec<_> = world
        .query::<&Press>()
        .iter(&world)
        .filter_map(|press| match press {
            Press::Echoes(Action::SellCard(card)) => Some(*card),
            _ => None,
        })
        .collect();
    assert_eq!(sell_buttons, vec![spare_card]);

    let before = ui.model.run().unwrap().clone();
    let work = super::super::super::choices::act(&mut ui, Action::SellCard(spare_card))
        .expect("revision-bound sell card job");
    assert!(matches!(
        work.request(),
        Some(api::Request::Command {
            run: id,
            revision,
            command: Command::SellCard { card },
            ..
        }) if *id == before.id && *revision == before.revision && *card == spare_card
    ));
    assert_eq!(
        ui.model.run(),
        Some(&before),
        "wallet and card collection wait for server confirmation"
    );
}

#[test]
#[allow(clippy::too_many_lines)] // End-to-end Reward dialog check across cursor, claim label, gold breakdown, and deck preview.
fn reward_dialog_shows_pointer_cursor_progress_claim_label_gold_breakdown_and_deck_preview() {
    let mut ui = opened();
    let cohort = ui.model.publication().unwrap().clone();
    let mut run = ui.model.run().unwrap().clone();
    let candidates: Vec<_> = cohort
        .catalog
        .cards
        .iter()
        .filter(|entry| !entry.basic_land)
        .take(5)
        .map(|entry| entry.card)
        .collect();
    // Mark first candidate as already owned to verify duplicate +Xg badge & breakdown.
    if run.cards.copies(candidates[0]) == 0 {
        run.cards.owned.push(domain::cards::Stack {
            card: candidates[0],
            copies: 1,
        });
    }
    run.phase = api::Phase::Reward;
    run.history.push(api::Duel {
        outcome: domain::v2::run::Outcome::Victory,
        turns: 9,
        used: [0; 2],
        bonus_gold: 3,
        claim: None,
    });
    run.reward = Some(api::Reward {
        cards: candidates.clone(),
        take: 3,
    });
    run.revision += 1;
    let read = ui.model.read(Uuid::now_v7(), Read::Run(run.id)).unwrap();
    ui.model.receive(
        &read,
        Some(api::Response::Run {
            run: Box::new(run.clone()),
        }),
        None,
    );
    ui.choices.bind(ui.model.run().unwrap());
    ui.reward_gold_open = true;
    ui.reward_deck_open = true;

    let render_reward = |ui: &Ui| {
        let mut world = World::new();
        let fonts = crate::hud::UiFonts::default();
        let kit = test_kit(&fonts, true);
        let mut commands = world.commands();
        let root = commands.spawn(Node::default()).id();
        super::super::reward::draw(
            &mut commands,
            root,
            kit,
            ui,
            ui.model.run().unwrap(),
            &Scrolled::default(),
        );
        world.flush();
        world
    };

    let mut world = render_reward(&ui);
    // 1. Every candidate tile is a live `Role::Hit` without `Disabled` -> Pointer cursor!
    let tiles: Vec<_> = world
        .query::<(
            Entity,
            &Press,
            &crate::shellkit::Role,
            Option<&controls::Disabled>,
        )>()
        .iter(&world)
        .filter(|(_, press, _, _)| matches!(press, Press::Echoes(Action::Reward(_))))
        .collect();
    assert_eq!(tiles.len(), 5);
    for (_, _, role, disabled) in &tiles {
        assert_eq!(**role, crate::shellkit::Role::Hit);
        assert!(disabled.is_none(), "selectable reward tile must be live");
    }

    // 2. No noisy "Wählen" pill, Claim button embeds `(0 / 2)` directly, Gold breakdown & Curve show.
    let texts: Vec<String> = world
        .query::<&Text>()
        .iter(&world)
        .map(|t| t.0.clone())
        .collect();
    assert!(
        !texts
            .iter()
            .any(|t| t == Phrase::EchoesRewardSelect.text(Lang::De)),
        "unselected reward cards must not render a noisy Wählen tag"
    );
    assert!(
        texts.iter().any(|t| t.contains("0 / 3")),
        "disabled Claim button embeds selection progress (0 / 3)"
    );
    assert!(
        texts
            .iter()
            .any(|t| t == Phrase::EchoesRewardSourceDuel.text(Lang::De)),
        "gold breakdown lists duel completion source"
    );
    assert!(
        texts
            .iter()
            .any(|t| t == Phrase::EchoesRewardCurveLabel.text(Lang::De)),
        "deck preview renders mana curve"
    );

    // 3. Select 3 cards -> Claim button flips to live ceremonial label.
    assert!(super::super::super::choices::act(&mut ui, Action::Reward(candidates[0])).is_none());
    assert!(super::super::super::choices::act(&mut ui, Action::Reward(candidates[1])).is_none());
    assert!(super::super::super::choices::act(&mut ui, Action::Reward(candidates[2])).is_none());
    let mut world = render_reward(&ui);
    let claim = world
        .query::<(&Press, Option<&controls::Disabled>)>()
        .iter(&world)
        .find(|(press, _)| **press == Press::Echoes(Action::ClaimReward))
        .expect("claim button");
    assert!(claim.1.is_none(), "claim button becomes live at 3 / 3");
    let texts: Vec<String> = world
        .query::<&Text>()
        .iter(&world)
        .map(|t| t.0.clone())
        .collect();
    assert!(
        texts
            .iter()
            .any(|t| t == Phrase::EchoesRewardClaim.text(Lang::De))
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

#[test]
fn commander_editor_controls_move_owned_card_to_sideboard_and_save_all_zones_together() {
    use baylee_client_core::echoes::draft::DeckZone;
    let mut ui = opened();
    let run = ui.model.run().unwrap().clone();
    let cohort = ui.model.publication().unwrap().clone();
    let spell = run
        .cards
        .deck
        .iter()
        .find(|row| !cohort.catalog.card(row.card).unwrap().basic_land)
        .unwrap()
        .card;
    let basic = run
        .cards
        .deck
        .iter()
        .find(|row| cohort.catalog.card(row.card).unwrap().basic_land)
        .unwrap()
        .card;
    let act = super::super::super::choices::act;
    assert!(act(&mut ui, Action::ZoneCard(spell, DeckZone::Main, false)).is_none());
    assert!(act(&mut ui, Action::ZoneCard(spell, DeckZone::Sideboard, true)).is_none());
    assert!(
        act(&mut ui, Action::SaveDeck).is_none(),
        "38-card library cannot be saved"
    );
    assert!(act(&mut ui, Action::ZoneCard(basic, DeckZone::Main, true)).is_none());
    let work = act(&mut ui, Action::SaveDeck).expect("complete legal formation");
    assert!(
        matches!(work.request(), Some(api::Request::Command { command: Command::Refit { deck, formation }, .. })
        if deck.iter().map(|row| u16::from(row.copies)).sum::<u16>() == 39
        && formation.commander == run.cards.formation.as_ref().unwrap().commander
        && formation.sideboard.len() == 1 && formation.sideboard[0].card == spell)
    );
    assert_eq!(
        ui.model.run(),
        Some(&run),
        "confirmed deck remains unchanged until response"
    );
}
