use super::*;
use baylee_client_core::echoes::domain::v2::catalog::{Definition, Family, Potion};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::text::TextSpan;

fn test_kit(fonts: &crate::hud::UiFonts, german: bool) -> Kit<'_> {
    Kit {
        fonts,
        german,
        m: crate::shellkit::ShellMetrics::of(
            crate::shellkit::Viewport::desktop(1280.0, 800.0),
            crate::shellkit::TextSize::default(),
        ),
    }
}

#[test]
fn enabled_item_details_follow_keyboard_focus_remain_idle_and_leave_no_orphan() {
    let (mut app, fonts) = crate::shellkit::header::tests::layout_app();
    app.init_resource::<InputFocus>()
        .insert_resource(bevy::input_focus::InputFocusVisible(true))
        .add_systems(Update, controls::show_tooltips);
    let kit = test_kit(&fonts, false);
    let owner = button(
        &mut app.world_mut().commands(),
        kit,
        Item {
            goods: Goods::Relic(Definition {
                family: Family::GildedAcorn,
                rarity: Rarity::MythicRare,
            }),
            name: "Gilded Acorn · Mythic Rare",
            description: "Earn +4 gold. Grow Plants. Strengthen your tokens.",
            caption: Some("×3"),
            hint: None,
            size: 80.0,
            emphasis: Emphasis::Mythic,
            selected: false,
        },
        Live::Yes,
        (),
    );
    app.world_mut().flush();
    app.update();
    app.update();
    let tip = app.world().get::<controls::Tooltip>(owner).unwrap().0;
    assert!(app.world().get::<controls::Disabled>(owner).is_none());
    assert!(app.world().get::<ChildOf>(tip).is_none());

    // Header separates clean primary Name from secondary Rarity subline (never mashed).
    let texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "Gilded Acorn"));
    assert!(texts.iter().any(|t| t == "Mythic"));
    assert!(!texts.iter().any(|t| t.contains("Gilded Acorn ·")));

    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(owner, FocusCause::Navigated);
    app.update();
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(tip),
        Some(&Visibility::Visible)
    );
    let last = app
        .world()
        .entity(tip)
        .get_ref::<Node>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .entity(tip)
            .get_ref::<Node>()
            .unwrap()
            .last_changed(),
        last
    );
    // A pointer click retains the focus target, but must not pin a hover detail
    // over later shop interactions after the pointer leaves the item.
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocusVisible>()
        .0 = false;
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(tip),
        Some(&Visibility::Hidden)
    );
    app.world_mut().despawn(owner);
    app.update();
    assert!(
        app.world().get_entity(tip).is_err(),
        "rich detail root is owned by the item"
    );
}

#[test]
fn once_per_turn_badge_appears_only_on_capped_items_and_command_zone_boilerplate_is_stripped() {
    // Potions and unlimited Mythic relics have no per-turn cap.
    assert_eq!(
        words::activation_limit_tag(Goods::Potion(Potion::Healing), Lang::De),
        None
    );
    assert_eq!(
        words::activation_limit_tag(
            Goods::Relic(Definition {
                family: Family::BloodAmber,
                rarity: Rarity::MythicRare,
            }),
            Lang::De,
        ),
        None
    );
    // Common/Rare Blood Amber is capped at 1x per turn.
    let capped = Goods::Relic(Definition {
        family: Family::BloodAmber,
        rarity: Rarity::Common,
    });
    assert_eq!(
        words::activation_limit_tag(capped, Lang::De).as_deref(),
        Some("1× pro Zug")
    );
    assert_eq!(
        words::activation_limit_tag(capped, Lang::En).as_deref(),
        Some("Once per turn")
    );

    // Neither Relic descriptions nor Potion token abilities mention the command zone.
    for lang in [Lang::En, Lang::De] {
        let desc = words::description(capped, lang);
        assert!(!desc.to_lowercase().contains("command zone"));
        assert!(!desc.to_lowercase().contains("kommandozone"));
        for def in [
            &baylee_cards::tokens::TOWER_HEALING_POTION,
            &baylee_cards::tokens::TOWER_MANA_POTION,
        ] {
            let line = words::token_ability(baylee_cards::tokens::token_id(def), 0, lang).unwrap();
            assert!(!line.to_lowercase().contains("command zone"));
            assert!(!line.to_lowercase().contains("kommandozone"));
        }
    }
}

#[test]
fn rich_prose_preserves_exact_bytes_and_highlights_life_gold_charges_and_damage() {
    let sample_de = "Ausgerüstet: +4 Start-LeP. Entferne 2 Ladungsmarken: 3 Schaden und +2 Gold. Nur einmal pro Zug aktivierbar.";
    let spans = prose::stylize(sample_de);
    let rebuilt: String = spans.iter().map(|s| s.text).collect();
    assert_eq!(rebuilt, sample_de);
    assert!(
        spans
            .iter()
            .any(|s| s.tone == prose::Tone::Life && s.text.contains("+4"))
    );
    assert!(
        spans
            .iter()
            .any(|s| s.tone == prose::Tone::Charge && s.text.contains("Ladungsmarken"))
    );
    assert!(
        spans
            .iter()
            .any(|s| s.tone == prose::Tone::Damage && s.text.contains("Schaden"))
    );
    assert!(
        spans
            .iter()
            .any(|s| s.tone == prose::Tone::Gold && s.text.contains("Gold"))
    );
    assert!(spans.iter().any(|s| s.tone == prose::Tone::Limit));

    // Exhaustively verify lossless UTF-8 reconstruction across every catalog item in EN & DE.
    for lang in [Lang::En, Lang::De] {
        for family in Family::ALL {
            for rarity in Rarity::ALL {
                let desc = words::description(Goods::Relic(Definition { family, rarity }), lang);
                for line in desc.lines() {
                    let s = prose::stylize(line);
                    let joined: String = s.iter().map(|part| part.text).collect();
                    assert_eq!(joined, line, "{family:?} {rarity:?} {lang:?}");
                }
            }
        }
    }
}

#[test]
fn attach_item_tooltip_and_frame_wire_ornate_decor_and_reusable_overlay() {
    let mut world = World::new();
    let fonts = crate::hud::UiFonts::default();
    let kit = test_kit(&fonts, true);
    let goods = Goods::Relic(Definition {
        family: Family::SpringChalice,
        rarity: Rarity::Rare,
    });
    let (panel, frost, target, tip) = {
        let mut commands = world.commands();
        let root = commands.spawn(Node::default()).id();
        let panel = frame(&mut commands, root, kit);
        let frost = shelf_frost(&mut commands, panel);
        let target = commands.spawn(Node::default()).id();
        let tip = attach_item_tooltip(&mut commands, target, kit, goods, Lang::De);
        (panel, frost, target, tip)
    };
    world.flush();
    assert!(world.get::<Children>(panel).is_some());
    assert!(world.get::<scenery::glint::FrostSurface>(frost).is_some());
    assert_eq!(world.get::<controls::Tooltip>(target).unwrap().0, tip);
    assert!(world.get::<controls::FocusTooltip>(target).is_some());
    assert!(world.query::<&TextSpan>().iter(&world).count() > 0);
    assert!(words::lookup_by_name(&words::name(goods, Lang::De)).is_some());
}

/// Every level step of every item, in both languages, shows something: a
/// changed or a new line. And no level drops a line of the level before,
/// because the ladder draws what a level says and would hide a dropped one.
#[test]
fn every_level_of_every_item_shows_what_it_changes_and_drops_nothing() {
    use baylee_client_core::echoes::ladder::{Change, step};
    for lang in [Lang::En, Lang::De] {
        for family in Family::ALL {
            let levels: Vec<String> = Rarity::ALL
                .iter()
                .map(|&rarity| words::run_effect(Definition { family, rarity }, lang))
                .collect();
            for (before, now) in levels.iter().zip(&levels[1..]) {
                let before: Vec<&str> = before.lines().collect();
                let now: Vec<&str> = now.lines().collect();
                let step = step(&before, &now);
                assert!(
                    step.lines.iter().any(|line| line.change != Change::Same),
                    "{family:?} {lang:?}: a level that changes nothing: {now:?}"
                );
                assert!(
                    step.removed.is_empty(),
                    "{family:?} {lang:?}: a level drops {:?}",
                    step.removed
                );
            }
        }
    }
}

/// A test app with the controls, the ladder and real fonts, laid out in a
/// 1280 × 720 window (the overlay's `max_width` is in viewport units).
fn ladder_app() -> (App, crate::hud::UiFonts) {
    use bevy::camera::{ComputedCameraValues, RenderTargetInfo};
    let (mut app, fonts) = crate::shellkit::header::tests::layout_app();
    app.world_mut().spawn((
        Window {
            resolution: bevy::window::WindowResolution::new(1280, 720)
                .with_scale_factor_override(1.0),
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    app.world_mut().spawn((
        Camera2d,
        bevy::ui::IsDefaultUiCamera,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(1280, 720),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
    ));
    app.init_resource::<InputFocus>()
        .insert_resource(bevy::input_focus::InputFocusVisible(true))
        .insert_resource(fonts.clone());
    controls::install(&mut app);
    ladder::install(&mut app);
    (app, fonts)
}

/// An item's overlay, shown by keyboard focus on its owner.
fn focused_overlay(app: &mut App, fonts: &crate::hud::UiFonts, goods: Goods) -> (Entity, Entity) {
    let kit = test_kit(fonts, true);
    let owner = app
        .world_mut()
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: px(200.0),
            // Low on the canvas: a ladder grown under it must flip above.
            top: px(560.0),
            width: px(64.0),
            height: px(64.0),
            ..default()
        })
        .id();
    let tip = attach_item_tooltip(&mut app.world_mut().commands(), owner, kit, goods, Lang::De);
    app.world_mut().flush();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(owner, FocusCause::Navigated);
    app.update();
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(tip),
        Some(&Visibility::Visible)
    );
    (owner, tip)
}

fn shift(app: &mut App, down: bool) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    if down {
        keys.press(KeyCode::ShiftLeft);
    } else {
        keys.release(KeyCode::ShiftLeft);
    }
}

fn slot_of(app: &mut App, tip: Entity) -> Entity {
    app.world()
        .get::<Children>(tip)
        .unwrap()
        .iter()
        .find(|child| app.world().get::<ladder::Ladder>(*child).is_some())
        .expect("a relic's overlay has a ladder slot")
}

fn size_of(app: &App, tip: Entity) -> Vec2 {
    let node = app.world().get::<bevy::ui::ComputedNode>(tip).unwrap();
    node.size() * node.inverse_scale_factor()
}

/// The ticks at which every `Node`, `Text` and `Visibility` under `tip` last
/// changed.
fn writes(app: &mut App, tip: Entity) -> Vec<(Entity, u32, u32, u32)> {
    let world = app.world_mut();
    let mut all: Vec<Entity> = vec![tip];
    let mut at = 0;
    while at < all.len() {
        if let Some(children) = world.get::<Children>(all[at]) {
            all.extend(children.iter());
        }
        at += 1;
    }
    all.into_iter()
        .map(|entity| {
            let entity_ref = world.entity(entity);
            let tick = |changed: Option<bevy::ecs::change_detection::Tick>| {
                changed.map_or(0, bevy::ecs::change_detection::Tick::get)
            };
            (
                entity,
                tick(
                    entity_ref
                        .get_ref::<Node>()
                        .map(|changed| changed.last_changed()),
                ),
                tick(
                    entity_ref
                        .get_ref::<Text>()
                        .map(|changed| changed.last_changed()),
                ),
                tick(
                    entity_ref
                        .get_ref::<Visibility>()
                        .map(|changed| changed.last_changed()),
                ),
            )
        })
        .collect()
}

#[test]
fn held_shift_opens_the_level_ladder_in_place_and_release_closes_it() {
    let (mut app, fonts) = ladder_app();
    let goods = Goods::Relic(Definition {
        family: Family::HarvestWheel,
        rarity: Rarity::Uncommon,
    });
    let (owner, tip) = focused_overlay(&mut app, &fonts, goods);
    let slot = slot_of(&mut app, tip);
    assert_eq!(
        app.world().get::<Node>(slot).unwrap().display,
        Display::None
    );
    assert!(
        app.world().get::<Children>(slot).is_none(),
        "built on first Shift"
    );
    let texts = |app: &mut App| -> Vec<String> {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect()
    };
    assert!(texts(&mut app).iter().any(|t| t == "Stufen vergleichen"));
    let rest = size_of(&app, tip);

    // At rest nothing is written, frame after frame.
    let before = writes(&mut app, tip);
    app.update();
    app.update();
    assert_eq!(
        writes(&mut app, tip),
        before,
        "the overlay at rest writes nothing"
    );

    shift(&mut app, true);
    app.update();
    assert_eq!(
        app.world().get::<Node>(slot).unwrap().display,
        Display::Flex
    );
    assert_eq!(
        app.world().get::<Visibility>(tip),
        Some(&Visibility::Visible),
        "shown again in the frame it grew"
    );
    assert!(app.world().resource::<crate::flip::ShiftClaimed>().0);
    let open = size_of(&app, tip);
    assert!(open.y > rest.y + 100.0, "{rest:?} → {open:?}");
    let all = texts(&mut app);
    for head in ["Stufe 1", "Stufe 2", "Stufe 3", "Stufe 4", "aktuell", "+"] {
        assert!(all.iter().any(|t| t == head), "{head} in {all:?}");
    }
    // Placed from its new size in that same frame: the next frame does not
    // move it.
    let placed = app
        .world()
        .get::<UiGlobalTransform>(tip)
        .unwrap()
        .translation;
    // Inside the 1280 × 720 window by the tooltip's inset.
    let (top_left, bottom_right) = (placed - open / 2.0, placed + open / 2.0);
    assert!(top_left.cmpge(Vec2::splat(9.5)).all(), "{top_left:?}");
    assert!(
        bottom_right.cmple(Vec2::new(1280.0, 720.0) - 9.5).all(),
        "{bottom_right:?}"
    );
    app.update();
    let next = app
        .world()
        .get::<UiGlobalTransform>(tip)
        .unwrap()
        .translation;
    assert!(
        (next - placed).length() < 0.01,
        "{placed:?} jumped to {next:?}"
    );

    // Held, it rests too.
    let held = writes(&mut app, tip);
    app.update();
    app.update();
    assert_eq!(
        writes(&mut app, tip),
        held,
        "the open ladder at rest writes nothing"
    );

    shift(&mut app, false);
    app.update();
    assert_eq!(
        app.world().get::<Node>(slot).unwrap().display,
        Display::None
    );
    assert_eq!(
        app.world().get::<Visibility>(tip),
        Some(&Visibility::Visible)
    );
    assert!(!app.world().resource::<crate::flip::ShiftClaimed>().0);
    assert!((size_of(&app, tip).y - rest.y).abs() < 0.5);

    // Built once: a second Shift shows the same ladder.
    let built = app.world().get::<Children>(slot).unwrap().len();
    shift(&mut app, true);
    app.update();
    assert_eq!(app.world().get::<Children>(slot).unwrap().len(), built);
    app.world_mut().despawn(owner);
}

#[test]
fn what_has_no_levels_wears_no_hint_and_no_ladder() {
    let (mut app, fonts) = ladder_app();
    let (_, tip) = focused_overlay(&mut app, &fonts, Goods::Potion(Potion::Healing));
    assert!(app.world().get::<ladder::Levels>(tip).is_none());
    let mut hints = app.world_mut().query::<&ladder::LevelsHint>();
    assert_eq!(hints.iter(app.world()).count(), 0);
    shift(&mut app, true);
    app.update();
    assert!(!app.world().resource::<crate::flip::ShiftClaimed>().0);
}

/// Every item's ladder, in German (the longer language) and at the top level
/// (the longest description over it), stays inside a 1280 × 720 window with
/// the tooltip's inset.
#[test]
fn every_ladder_fits_a_1280_by_720_window() {
    let (mut app, fonts) = ladder_app();
    shift(&mut app, true);
    let mut tallest = (0.0, None);
    for family in Family::ALL {
        let definition = Definition {
            family,
            rarity: Rarity::MythicRare,
        };
        let (owner, tip) = focused_overlay(&mut app, &fonts, Goods::Relic(definition));
        let open = size_of(&app, tip);
        assert!(
            open.x > ladder::REST_WIDTH,
            "{definition:?} opened: {open:?}"
        );
        if open.y > tallest.0 {
            tallest = (open.y, Some(definition));
        }
        app.world_mut().despawn(owner);
    }
    assert!(tallest.0 <= 720.0 - 2.0 * 10.0, "tallest: {tallest:?}");
}
