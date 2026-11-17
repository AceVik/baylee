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
    assert!(texts.iter().any(|t| t == "Mythic Rare"));
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
