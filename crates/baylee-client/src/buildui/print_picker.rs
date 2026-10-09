//! Artwork selection and searchable printing catalog.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::hud::{btn_radius, palette};
use crate::lobby::{FieldLook, List, SharedPress, button, chip, note, row, spacer, text_field};
use baylee_client_core::deckbuilder::{BuildField, Picker, PoolCard};

/// The printing picker: the carousel, the language, the finish.
///
/// Drawn over the whole builder rather than beside it, on every frame size.
/// This is one question with one answer — which piece of cardboard — and a
/// panel wedged next to a card list would be the narrowest place to look at
/// art on a phone, which is the one thing this dialog exists to show.
#[allow(clippy::too_many_lines)] // one dialog, six rows, each trivial
#[allow(clippy::too_many_arguments)] // one dialog: the tree, the state, the stores
pub(crate) fn printing_picker(
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
            Press::Build(BuildPress::PickerClose),
            ZIndex(20),
        ))
        .id();

    let panel = commands
        .spawn((
            Node {
                width: percent(100),
                max_height: percent(100),
                overflow: Overflow::scroll_y(),
                max_width: px(if metrics.frame == Frame::Compact {
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
            Press::Shared(SharedPress::PickerNothing),
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
        Press::Build(BuildPress::PickerClose),
        false,
    );
    let refresh = icon_button(
        commands,
        fonts,
        metrics,
        "\u{f2f1}",
        Press::Build(BuildPress::PickerRefresh),
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
        Press::Build(BuildPress::PickerStep(-1)),
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
    let art = picker_art(
        commands,
        fonts,
        metrics,
        lang,
        picker,
        deck.card(picker.slot()),
        assets,
        cards,
    );
    let forward = icon_button(
        commands,
        fonts,
        metrics,
        "\u{f054}",
        Press::Build(BuildPress::PickerStep(1)),
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
            // The registry's own reference printing knows nothing but its
            // id: offline, and wherever the printings could not be read.
            let mut line = printing
                .label()
                .unwrap_or_else(|| Phrase::ReferencePrinting.text(lang).to_string());
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
                index: None,
            },
        );
        commands.entity(thumb).insert((
            Press::Build(BuildPress::PickerGo(at)),
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
            Press::Build(BuildPress::PickerLang(None)),
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
                Press::Build(BuildPress::PickerLang(Some(i))),
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
        Press::Build(BuildPress::PickerForceFinish),
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
            Press::Build(BuildPress::PickerFinish(finish)),
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
        Press::Build(BuildPress::PickerConfirm),
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

/// The art for the printing the carousel is on, or the card's text face
/// where there is none to show.
///
/// The URL is built the same way the duel builds one, so a printing that
/// renders on the table renders here. Where there is no picture — no
/// plausible Scryfall id, a load that already failed, or one that fails
/// later ([`face_lost_art`]) — the card is drawn as its text face, from the
/// compiled card and the English Oracle where the pool has no text: a client
/// with no way out to Scryfall (offline, signed in nowhere) has nothing else
/// to show, and an empty frame with a cross was what it showed (the owner's
/// beta.6 review).
#[allow(clippy::too_many_arguments)] // one window part: the card, its printing, the stores
fn picker_art(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    picker: &Picker,
    card: Option<&PoolCard>,
    assets: Option<&AssetServer>,
    cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let height = if metrics.frame == Frame::Compact {
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
    let Some(cards) = cards else {
        let empty = note(
            commands,
            fonts,
            metrics,
            Phrase::NoArtForPrinting.text(lang),
        );
        commands.entity(holder).add_child(empty);
        return holder;
    };
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
    commands.entity(holder).add_child(frame);
    let width = height * baylee_client_core::layout::CARD_ASPECT;
    let finish = treatment(picker.finish());

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
    let art = url
        .zip(assets)
        .map(|(url, assets)| (assets.load::<Image>(url.clone()), url, assets))
        .filter(|(handle, _, assets)| !lost(assets, handle));
    let Some((handle, url, assets)) = art else {
        if let Some(card) = card {
            let face = crate::face::of_pool(card);
            let widths = crate::face::Widths::of(None);
            crate::face::spawn_ui_card(
                commands, cards, frame, lang, &face, fonts, &widths, finish, width,
            );
        } else {
            commands.entity(frame).despawn();
            let empty = note(
                commands,
                fonts,
                metrics,
                Phrase::NoArtForPrinting.text(lang),
            );
            commands.entity(holder).add_child(empty);
        }
        return holder;
    };
    sides(
        commands,
        cards,
        assets,
        frame,
        picker,
        (url, handle.clone()),
        finish,
    );
    if let Some(card) = card {
        commands.entity(frame).insert(AwaitingArt {
            art: handle,
            card: card.index,
            finish,
            width,
        });
    }
    holder
}

/// The picture's two sides in `frame`: the printing's front, and its back
/// or the card back, turned by holding Shift.
fn sides(
    commands: &mut Commands,
    cards: &mut UiCards<'_>,
    assets: &AssetServer,
    frame: Entity,
    picker: &Picker,
    (url, handle): (String, Handle<Image>),
    finish: FinishTreatment,
) {
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
        let art = if side == crate::flip::Side::Front {
            handle.clone()
        } else {
            assets.load(url.clone())
        };
        let material = cards.preview(&url, finish, art);
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
}

/// Whether a picture's load has failed: with no way out to Scryfall, every
/// one does.
fn lost(assets: &AssetServer, art: &Handle<Image>) -> bool {
    matches!(
        assets.get_load_state(art.id()),
        Some(bevy::asset::LoadState::Failed(_))
    )
}

/// A picker's art still on its way, and the card to draw as its text face
/// if it never arrives.
#[derive(Component)]
pub(crate) struct AwaitingArt {
    art: Handle<Image>,
    /// The pool card's registry index.
    card: u32,
    finish: FinishTreatment,
    /// The card's width, in logical pixels.
    width: f32,
}

/// Turns a picker's card whose picture failed to load into its text face
/// ([`picker_art`]), in place: the window stays as it is drawn, only the
/// card in it changes. A picture that arrives is left alone.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(crate) fn face_lost_art(
    mut commands: Commands,
    waiting: Query<(Entity, &AwaitingArt)>,
    server: Option<Res<AssetServer>>,
    images: Option<Res<Assets<Image>>>,
    state: Res<LobbyState>,
    fonts: Option<Res<UiFonts>>,
    font_assets: Option<Res<Assets<Font>>>,
    (mut cache, mut store): (
        Option<ResMut<crate::cardmat::UiCardMaterials>>,
        Option<ResMut<Assets<crate::cardmat::CardUiMaterial>>>,
    ),
) {
    // A headless lobby has no asset server, and draws no picture to lose.
    let (Some(server), Some(images), Some(fonts)) = (server, images, fonts) else {
        return;
    };
    for (frame, awaiting) in &waiting {
        if images.contains(awaiting.art.id()) {
            commands.entity(frame).remove::<AwaitingArt>();
            continue;
        }
        if !lost(&server, &awaiting.art) {
            continue;
        }
        let (Some(cache), Some(store)) = (cache.as_deref_mut(), store.as_deref_mut()) else {
            return;
        };
        let lobby = &state.lobby;
        let Some(card) = lobby
            .builder()
            .pool()
            .iter()
            .find(|c| c.index == awaiting.card)
        else {
            commands.entity(frame).remove::<AwaitingArt>();
            continue;
        };
        let face = crate::face::of_pool(card);
        let widths =
            crate::face::Widths::of(font_assets.as_deref().and_then(|a| a.get(&fonts.text)));
        commands
            .entity(frame)
            .remove::<AwaitingArt>()
            .despawn_related::<Children>();
        let mut cards = UiCards {
            cache,
            assets: store,
        };
        crate::face::spawn_ui_card(
            &mut commands,
            &mut cards,
            frame,
            lobby.lang(),
            &face,
            &fonts,
            &widths,
            awaiting.finish,
            awaiting.width,
        );
    }
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

pub(super) fn set_search(
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
            press: Press::Build(BuildPress::FocusBuild(BuildField::PickerSet)),
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
            Press::Shared(SharedPress::PickerNothing),
        ))
        .id();
    let all = chip(
        commands,
        fonts,
        metrics,
        Phrase::AllSets.text(lang),
        Press::Build(BuildPress::PickerSet(None)),
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
            Press::Build(BuildPress::PickerSet(Some(i))),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardmat::{CardUiMaterial, UiCardMaterials};
    use baylee_client_core::deckbuilder::Zone;

    /// Birds of Paradise as the pool knows it with no catalog (offline):
    /// no rules text of its own, so its face reads the compiled English
    /// Oracle. `scryfall_id` decides whether there is a picture to ask for.
    fn birds(scryfall_id: &str) -> PoolCard {
        PoolCard {
            index: baylee_cards::decks::by_name("Birds of Paradise")
                .expect("in the pool")
                .get(),
            name: "Birds of Paradise".to_string(),
            english_name: "Birds of Paradise".to_string(),
            colors: "G".to_string(),
            scryfall_id: scryfall_id.to_string(),
            ..PoolCard::default()
        }
    }

    /// Draws the picker's card for `card`, open on it, as the builder does,
    /// and lets `frames` frames pass: what stands in the window then.
    fn window(card: PoolCard, frames: usize) -> App {
        let mut app = App::new();
        // An asset server with no `https` source: every picture fails, as
        // it does on a machine with no way out to Scryfall.
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Font>()
            .init_asset::<CardUiMaterial>()
            .init_resource::<UiCardMaterials>()
            .init_resource::<LobbyState>()
            .insert_resource(UiFonts {
                text: Handle::default(),
                medium: Handle::default(),
                bold: Handle::default(),
                italic: Handle::default(),
                medium_italic: Handle::default(),
                serif: Handle::default(),
                serif_italic: Handle::default(),
                icons: Handle::default(),
                mana: Handle::default(),
            })
            .add_systems(Update, (draw, face_lost_art).chain());
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            let builder = state.lobby.builder_mut();
            builder.set_pool(vec![card], false);
            builder.open_picker(0, Zone::Main);
        }
        for _ in 0..frames {
            app.update();
        }
        app
    }

    /// The picker's card, drawn once.
    fn draw(
        mut commands: Commands,
        state: Res<LobbyState>,
        assets: Res<AssetServer>,
        fonts: Res<UiFonts>,
        (mut cache, mut store): (ResMut<UiCardMaterials>, ResMut<Assets<CardUiMaterial>>),
        mut drawn: Local<bool>,
    ) {
        if std::mem::replace(&mut *drawn, true) {
            return;
        }
        let deck = state.lobby.builder();
        let picker = deck.picker().expect("open");
        let mut cards = UiCards {
            cache: &mut cache,
            assets: &mut store,
        };
        picker_art(
            &mut commands,
            &fonts,
            Metrics::of(1400.0),
            Lang::En,
            picker,
            deck.card(picker.slot()),
            Some(&assets),
            Some(&mut cards),
        );
    }

    /// Whether the card in the window is its text face, with its rules
    /// text, and whether a picture is still asked to stand there.
    fn shown(app: &mut App) -> (bool, bool) {
        let world = app.world_mut();
        let mut texts: Vec<String> = world
            .query::<&Text>()
            .iter(world)
            .map(|t| t.0.clone())
            .collect();
        texts.extend(world.query::<&TextSpan>().iter(world).map(|t| t.0.clone()));
        let faced = world
            .query_filtered::<(), With<crate::face::FaceTextBox>>()
            .iter(world)
            .count()
            == 1
            && texts.iter().any(|t| t.contains("one mana of any color"));
        let fronts: Vec<_> = world
            .query::<(&crate::flip::Side, &MaterialNode<CardUiMaterial>)>()
            .iter(world)
            .filter(|(side, _)| **side == crate::flip::Side::Front)
            .map(|(_, node)| node.0.clone())
            .collect();
        let materials = world.resource::<Assets<CardUiMaterial>>();
        let pictured = fronts
            .iter()
            .any(|handle| materials.get(handle).is_some_and(|m| m.art.is_some()));
        (faced, pictured)
    }

    /// The owner's beta.6 review: offline, the card window a picture opens
    /// in the deck builder showed an empty frame with a cross. With no way
    /// out to Scryfall the picture never comes, and the card is drawn as
    /// its text face in its place, from the compiled card and the English
    /// Oracle.
    #[test]
    fn a_picture_that_never_comes_leaves_the_card_as_its_text_face() {
        let mut app = window(birds("2f40613b-1bde-4939-86ad-6bd40f9db0d6"), 1);
        assert_eq!(
            shown(&mut app),
            (false, true),
            "the picture is asked for first"
        );
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(
            shown(&mut app),
            (true, false),
            "the failed picture gives way to the text face"
        );
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<(), With<AwaitingArt>>()
                .iter(world)
                .count(),
            0,
            "and nothing waits for it any more"
        );
    }

    /// A printing with no picture to ask for (the registry's reference row,
    /// a nil id) is its text face from the first frame.
    #[test]
    fn a_printing_with_no_picture_is_its_text_face_at_once() {
        let mut app = window(birds("00000000-0000-0000-0000-000000000000"), 1);
        assert_eq!(shown(&mut app), (true, false));
    }
}
