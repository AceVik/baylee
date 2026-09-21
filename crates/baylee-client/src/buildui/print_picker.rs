//! Artwork selection and searchable printing catalog.
#[allow(clippy::wildcard_imports)]
use super::*;

/// The printing picker: the carousel, the language, the finish.
///
/// Drawn over the whole builder rather than beside it, on every frame size.
/// This is one question with one answer — which piece of cardboard — and a
/// panel wedged next to a card list would be the narrowest place to look at
/// art on a phone, which is the one thing this dialog exists to show.
#[allow(clippy::too_many_lines)] // one dialog, six rows, each trivial
#[allow(clippy::too_many_arguments)] // one dialog: the tree, the state, the stores
pub(super) fn printing_picker(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    deck: &DeckBuilder,
    picker: &Picker,
    assets: Option<&AssetServer>,
    cards: Option<&mut UiCards<'_>>,
    scrolled: &Scrolled,
) -> Entity {
    let shade = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(px(metrics.pad)),
                ..default()
            },
            BackgroundColor(palette::SHADOW.with_alpha(0.82)),
            // Tapping the dark outside puts it away — the same gesture every
            // dialog on a phone answers to.
            Press::PickerClose,
            ZIndex(20),
        ))
        .id();

    let panel = commands
        .spawn((
            Node {
                width: percent(100),
                max_height: percent(100),
                overflow: Overflow::scroll_y(),
                max_width: px(if metrics.frame == Frame::Phone {
                    520.0
                } else {
                    860.0
                }),
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap),
                padding: UiRect::all(px(metrics.pad)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(14)),
                ..default()
            },
            BackgroundColor(palette::PANEL.with_alpha(0.98)),
            BorderColor::all(palette::DOCK_EDGE),
            crate::hud::soft_shadow(),
            // Swallows the tap so a press inside the dialog is not also a
            // press on the shade behind it.
            Press::PickerNothing,
        ))
        .id();
    commands.entity(panel).insert((
        crate::lobby::Scrollable(List::PickerPanel),
        ScrollPosition(Vec2::new(0.0, scrolled.get(List::PickerPanel))),
    ));
    commands.entity(shade).add_child(panel);

    // ---- what card this is, and the way out
    let head = row(commands, metrics, false);
    let name = deck
        .card(picker.slot())
        .map_or_else(String::new, |c| c.name.clone());
    let title = commands
        .spawn((
            Text::new(name),
            tf(fonts, metrics.head),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    let close = icon_button(
        commands,
        fonts,
        metrics,
        "\u{f00d}",
        Press::PickerClose,
        false,
    );
    let refresh = icon_button(
        commands,
        fonts,
        metrics,
        "\u{f2f1}",
        Press::PickerRefresh,
        picker.loading(),
    );
    for child in [title, gap, refresh, close] {
        commands.entity(head).add_child(child);
    }
    commands.entity(panel).add_child(head);

    // ---- the carousel
    let stage = row(commands, metrics, false);
    commands.entity(stage).insert(Node {
        width: percent(100),
        align_items: AlignItems::Center,
        column_gap: px(metrics.gap),
        ..default()
    });
    let back = icon_button(
        commands,
        fonts,
        metrics,
        "\u{f053}",
        Press::PickerStep(-1),
        false,
    );
    commands.entity(back).insert(Node {
        width: px(52),
        height: px(68),
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        border_radius: btn_radius(),
        ..default()
    });
    let art = picker_art(commands, fonts, metrics, lang, picker, assets, cards);
    let forward = icon_button(
        commands,
        fonts,
        metrics,
        "\u{f054}",
        Press::PickerStep(1),
        false,
    );
    commands.entity(forward).insert(Node {
        width: px(52),
        height: px(68),
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        border_radius: btn_radius(),
        ..default()
    });
    for child in [back, art, forward] {
        commands.entity(stage).add_child(child);
    }
    commands.entity(panel).add_child(stage);

    // ---- which printing, in words
    let current = picker.current();
    let caption = match current {
        Some(printing) => {
            let mut line = printing.label();
            if !printing.set_name.is_empty() {
                line = format!("{} \u{2014} {line}", printing.set_name);
            }
            if !printing.artist.is_empty() {
                line = format!("{line}\n{}", printing.artist);
            }
            line
        }
        None => Phrase::NoPrintings.text(lang).to_string(),
    };
    let label = commands
        .spawn((
            Text::new(caption),
            TextLayout::justify(Justify::Center),
            Node {
                width: percent(100),
                ..default()
            },
            tf(fonts, metrics.small),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(label);

    let count = note(
        commands,
        fonts,
        metrics,
        &if picker.loading() {
            Phrase::LookingForPrintings.text(lang).to_string()
        } else if picker.from_catalog() {
            Phrase::PrintingAt.fill(
                lang,
                &[&(picker.at() + 1).to_string(), &picker.len().to_string()],
            )
        } else {
            // Saying so beats implying the card was printed exactly once.
            Phrase::NoCatalogOnlyThis.text(lang).to_string()
        },
    );
    let status = row(commands, metrics, false);
    commands.entity(status).insert(Node {
        width: percent(100),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        column_gap: px(8),
        min_height: px(22),
        ..default()
    });
    if picker.loading() {
        let spin = crate::card_loading::spinner(commands, 18.0);
        commands.entity(status).add_child(spin);
    }
    commands.entity(status).add_child(count);
    commands.entity(panel).add_child(status);

    // A bounded contact sheet lets people compare images without opening each one.
    let gallery = row(commands, metrics, false);
    commands.entity(gallery).insert(Node {
        width: percent(100),
        justify_content: JustifyContent::Center,
        column_gap: px(6),
        ..default()
    });
    let start = (picker.at() / 5) * 5;
    for (at, print) in picker.visible().iter().enumerate().skip(start).take(5) {
        let thumb = crate::lobby::thumbnails::spawn(
            commands,
            &crate::lobby::HoverCard {
                url: baylee_client_core::images::image_url(
                    &baylee_view::PrintEntry {
                        scryfall_id: print.scryfall_id.clone(),
                        lang: print.lang.clone(),
                        finish: baylee_view::Finish::Normal,
                    },
                    baylee_client_core::images::Face::Front,
                    baylee_client_core::images::ArtSize::Small,
                ),
                back_url: None,
                finish: treatment(picker.finish()),
            },
        );
        commands.entity(thumb).insert((
            Press::PickerGo(at),
            Pickable::default(),
            BorderColor::all(if at == picker.at() {
                palette::ACCENT
            } else {
                palette::DOCK_EDGE
            }),
            Node {
                width: px(48),
                height: px(67),
                border: UiRect::all(px(2)),
                flex_shrink: 0.0,
                ..default()
            },
        ));
        commands.entity(gallery).add_child(thumb);
    }
    commands.entity(panel).add_child(gallery);

    let sets = set_search(commands, fonts, metrics, lang, deck, picker);
    commands.entity(panel).add_child(sets);

    // ---- language
    if picker.langs().len() > 1 {
        let langs = row(commands, metrics, true);
        let all = chip(
            commands,
            fonts,
            metrics,
            if lang == Lang::De {
                "Alle Sprachen"
            } else {
                "All languages"
            },
            Press::PickerLang(None),
            picker.lang().is_none(),
        );
        commands.entity(langs).add_child(all);
        for (i, code) in picker.langs().iter().enumerate() {
            let on = picker.lang() == Some(code.as_str());
            let c = chip(
                commands,
                fonts,
                metrics,
                &code.to_uppercase(),
                Press::PickerLang(Some(i)),
                on,
            );
            commands.entity(langs).add_child(c);
        }
        commands.entity(panel).add_child(langs);
    }

    let force = chip(
        commands,
        fonts,
        metrics,
        if lang == Lang::De {
            "Foil / Etched erzwingen"
        } else {
            "Enforce foil / etched"
        },
        Press::PickerForceFinish,
        picker.force_finish(),
    );
    let check = commands
        .spawn((
            Node {
                width: px(18),
                height: px(18),
                flex_shrink: 0.0,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderColor::all(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    if picker.force_finish() {
        commands.entity(check).with_child((
            Text::new("\u{f00c}"),
            crate::hud::icon_tf(fonts, 12.0),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ));
    }
    commands.entity(force).insert_children(0, &[check]);

    // ---- finish
    let finishes = row(commands, metrics, true);
    let offered = picker.finishes();
    for (finish, label) in [
        (Finish::Normal, Phrase::FinishPlain.text(lang)),
        (Finish::Foil, Phrase::FinishFoil.text(lang)),
        (Finish::Etched, Phrase::FinishEtched.text(lang)),
        (Finish::Holographic, Phrase::FinishHolographic.text(lang)),
        (Finish::Glitter, Phrase::FinishGlitter.text(lang)),
        (Finish::Galaxy, Phrase::FinishGalaxy.text(lang)),
    ] {
        let sold = offered.contains(&finish);
        let c = button(
            commands,
            fonts,
            metrics,
            label,
            Press::PickerFinish(finish),
            if sold && picker.finish() == finish {
                palette::ACCENT
            } else {
                palette::PANEL_LIT
            },
            sold,
        );
        // A finish this printing was never sold in is shown dead rather than
        // hidden: which finishes exist is part of what a player is choosing
        // between, and a row of buttons that changes length as the carousel
        // moves is harder to read than one that greys out.
        if !sold {
            commands.entity(c).remove::<Press>();
            commands.entity(c).insert(Pickable::IGNORE);
            commands.entity(c).insert(BackgroundColor(Color::NONE));
        }
        commands.entity(finishes).add_child(c);
    }
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    commands.entity(finishes).add_children(&[gap, force]);
    commands.entity(panel).add_child(finishes);

    // ---- the way in
    let foot = row(commands, metrics, false);
    let held = note(
        commands,
        fonts,
        metrics,
        &Phrase::CountInZone.fill(
            lang,
            &[
                &deck.count_of(picker.slot(), picker.zone()).to_string(),
                match picker.zone() {
                    Zone::Main => Phrase::ZoneDeck,
                    Zone::Side => Phrase::ZoneSideboard,
                }
                .text(lang),
            ],
        ),
    );
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    let add = chip(
        commands,
        fonts,
        metrics,
        if picker.replacing() {
            Phrase::ApplyPrinting
        } else {
            Phrase::AddPrinting
        }
        .text(lang),
        Press::PickerConfirm,
        true,
    );
    commands.entity(add).insert(Node {
        min_height: px(metrics.tap),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.4)),
        flex_shrink: 0.0,
        border_radius: btn_radius(),
        ..default()
    });
    for child in [held, gap, add] {
        commands.entity(foot).add_child(child);
    }
    commands.entity(panel).add_child(foot);
    let hint = note(
        commands,
        fonts,
        metrics,
        if lang == Lang::De {
            "Shift halten: Rückseite · ← → Artwork wechseln"
        } else {
            "Hold Shift: reverse side · ← → change artwork"
        },
    );
    commands.entity(panel).add_child(hint);

    shade
}

/// The art for the printing the carousel is on.
///
/// The URL is built the same way the duel builds one, so a printing that
/// renders on the table renders here. A printing whose id is not a plausible
/// Scryfall id — the registry's own reference row, in a build with no catalog
/// — gets a plain panel instead of a guaranteed 404.
fn picker_art(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    picker: &Picker,
    assets: Option<&AssetServer>,
    cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let height = if metrics.frame == Frame::Phone {
        260.0
    } else {
        360.0
    };
    let holder = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                height: px(height),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Pickable::IGNORE,
        ))
        .id();

    let url = picker.current().and_then(|printing| {
        baylee_client_core::images::image_url(
            &baylee_view::PrintEntry {
                scryfall_id: printing.scryfall_id.clone(),
                lang: printing.lang.clone(),
                finish: baylee_view::Finish::Normal,
            },
            baylee_client_core::images::Face::Front,
            baylee_client_core::images::ArtSize::Normal,
        )
    });
    if let (Some(url), Some(assets), Some(cards)) = (url, assets, cards) {
        let frame = commands
            .spawn((
                crate::flip::Flip::default(),
                Node {
                    height: percent(100),
                    aspect_ratio: Some(baylee_client_core::layout::CARD_ASPECT),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::all(px(12)),
                    ..default()
                },
                crate::hud::soft_shadow(),
                Pickable::IGNORE,
            ))
            .id();
        let back = picker
            .current()
            .filter(|p| p.has_back_image())
            .and_then(|p| {
                baylee_client_core::images::image_url(
                    &baylee_view::PrintEntry {
                        scryfall_id: p.scryfall_id.clone(),
                        lang: p.lang.clone(),
                        finish: baylee_view::Finish::Normal,
                    },
                    baylee_client_core::images::Face::Back,
                    baylee_client_core::images::ArtSize::Normal,
                )
            })
            .unwrap_or_else(|| {
                baylee_client_core::images::back_url(baylee_client_core::images::ArtSize::Normal)
            });
        for (url, side) in [
            (url, crate::flip::Side::Front),
            (back, crate::flip::Side::Back),
        ] {
            let material =
                cards.preview(&url, treatment(picker.finish()), assets.load(url.clone()));
            commands.entity(frame).with_child((
                MaterialNode(material),
                side,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                if side == crate::flip::Side::Back {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                },
                Pickable::IGNORE,
            ));
        }
        commands.entity(holder).add_child(frame);
    } else {
        let empty = note(
            commands,
            fonts,
            metrics,
            Phrase::NoArtForPrinting.text(lang),
        );
        commands.entity(holder).add_child(empty);
    }
    holder
}

fn icon_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    icon: &str,
    press: Press,
    loading: bool,
) -> Entity {
    let root = commands
        .spawn((
            Node {
                width: px(metrics.tap),
                height: px(metrics.tap),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
        ))
        .id();
    if loading {
        let spin = crate::card_loading::spinner(commands, 20.0);
        commands.entity(root).add_child(spin);
    } else {
        commands.entity(root).insert(press).with_child((
            Text::new(icon),
            crate::hud::icon_tf(fonts, 18.0),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ));
    }
    root
}

fn set_search(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    deck: &DeckBuilder,
    picker: &Picker,
) -> Entity {
    let sets = picker.sets();
    let hint = picker
        .set()
        .and_then(|selected| sets.iter().find(|(code, _)| *code == selected))
        .map_or_else(
            || Phrase::AllSets.text(lang).to_string(),
            |(code, name)| format!("{} · {name}", code.to_uppercase()),
        );
    let root = text_field(
        commands,
        fonts,
        metrics,
        if lang == Lang::De {
            "Set · Kürzel oder Name suchen"
        } else {
            "Set · search code or name"
        },
        &FieldLook {
            buffer: deck.buffer(BuildField::PickerSet),
            focused: picker.set_open(),
            mask: None,
            press: Press::FocusBuild(BuildField::PickerSet),
            lead: Some(crate::hud::glyph::MAGNIFIER),
            hint: Some(&hint),
            tail: None,
        },
    );
    if !picker.set_open() {
        return root;
    }
    let popup = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: percent(100),
                width: percent(100),
                max_height: px(160),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(4)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(palette::ACCENT),
            GlobalZIndex(80),
            crate::lobby::Scrollable(List::PickerSets),
            ScrollPosition(Vec2::new(
                0.0,
                (f32::from(u16::try_from(picker.set_cursor()).unwrap_or(u16::MAX)) * 34.0 + 76.0
                    - 160.0)
                    .max(0.0),
            )),
            Press::PickerNothing,
        ))
        .id();
    let all = chip(
        commands,
        fonts,
        metrics,
        Phrase::AllSets.text(lang),
        Press::PickerSet(None),
        picker.set().is_none(),
    );
    commands.entity(popup).add_child(all);
    for (cursor, i) in picker.matching_sets().into_iter().enumerate() {
        let (code, name) = sets[i];
        let label = format!("{} · {name}", code.to_uppercase());
        let item = chip(
            commands,
            fonts,
            metrics,
            &label,
            Press::PickerSet(Some(i)),
            cursor == picker.set_cursor(),
        );
        commands.entity(item).insert(Node {
            width: percent(100),
            min_height: px(34),
            flex_shrink: 0.0,
            padding: UiRect::axes(px(10), px(6)),
            align_items: AlignItems::Center,
            ..default()
        });
        commands.entity(popup).add_child(item);
    }
    commands.entity(root).add_child(popup);
    root
}
