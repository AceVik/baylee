//! The deck builder screen: the pool on one side, the deck on the other.
//!
//! Split out of `lobby.rs` for the same reason as `settingsui.rs` — it is a
//! screen, not a lobby, and the two together were four thousand lines with no
//! seam in the middle. Everything that *decides* is still in
//! [`baylee_client_core::deckbuilder`]; what is here is the node tree, and it
//! borrows the lobby's `Metrics`, `Press` and widget helpers so that a deck
//! row looks like a lobby row without a second copy of either.

use crate::cardmat::UiCards;
use crate::hud::{UiFonts, btn_radius, palette, tf};
use crate::lobby::{
    Frame, List, LobbyState, Metrics, Pane, Press, Scrolled, button, chip, hover_of_card,
    hover_of_entry, note, print_mark, row, scroller, spacer, text_box,
};
use baylee_client_core::deckbuilder::{
    BuildField, CURVE_BUCKETS, Coverage, DeckBuilder, Group, Picker, Zone,
};
use baylee_client_core::images::FinishTreatment;
use baylee_core::preset::Finish;
use bevy::prelude::*;
use bevy::ui::{percent, px};

/// How many result rows are drawn before the list stops and says how many it
/// left out.
///
/// The pool is small enough to send whole and to *filter* on every keystroke,
/// but not small enough to spawn whole: a few hundred rows is a few thousand
/// UI nodes, rebuilt on every letter typed. This is a drawing budget, not a
/// filter — the line above the list always names the real total.
const SHOWN_RESULTS: usize = 60;

/// The tallest a mana-curve bar gets, in logical pixels.
const CURVE_HEIGHT: f32 = 54.0;

/// The colours the identity filter offers, and the pips it counts.
const COLORS: [(char, &str); 6] = [
    ('W', "White"),
    ('U', "Blue"),
    ('B', "Black"),
    ('R', "Red"),
    ('G', "Green"),
    ('C', "Colourless"),
];

/// The card types worth a chip of their own. Anything else is reached
/// through the search box.
const KINDS: [&str; 7] = [
    "Creature",
    "Instant",
    "Sorcery",
    "Artifact",
    "Enchantment",
    "Planeswalker",
    "Land",
];

/// The deck builder: the pool on one side, the deck on the other.
#[allow(clippy::too_many_arguments)] // one screen: the tree, the state, the stores
pub(crate) fn builder(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
    assets: Option<&AssetServer>,
    cards: Option<&mut UiCards<'_>>,
) {
    let deck = state.lobby.builder();
    let phone = metrics.frame == Frame::Phone;
    let counts = deck.counts();

    let bar = build_bar(commands, state, fonts, metrics);
    commands.entity(root).add_child(bar);

    // A phone has room for one half at a time, and the switch has to say what
    // is in the other one — a deck count is the whole reason to look.
    if phone {
        let switch = row(commands, metrics, true);
        for (pane, label) in [
            (Pane::Cards, format!("Cards ({})", deck.results().len())),
            (
                Pane::Deck,
                format!("Deck ({} / {})", counts.main, counts.side),
            ),
        ] {
            let chosen = state.pane == pane;
            let tab = chip(
                commands,
                fonts,
                metrics,
                &label,
                Press::ShowPane(pane),
                chosen,
            );
            commands.entity(tab).insert(Node {
                flex_grow: 1.0,
                min_height: px(metrics.tap),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: btn_radius(),
                ..default()
            });
            commands.entity(switch).add_child(tab);
        }
        commands.entity(switch).insert(Node {
            width: percent(100),
            column_gap: px(metrics.gap),
            padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.4)),
            ..default()
        });
        commands.entity(root).add_child(switch);
    }

    let body = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Row,
                column_gap: px(metrics.pad),
                padding: UiRect::all(px(metrics.pad)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(body);

    if !phone || state.pane == Pane::Cards {
        let pool = pool_panel(commands, state, fonts, metrics, scrolled_to);
        commands.entity(body).add_child(pool);
    }
    if !phone || state.pane == Pane::Deck {
        let list = deck_panel(commands, state, fonts, metrics, scrolled_to);
        commands.entity(body).add_child(list);
    }

    // Last, so it sits over both halves whatever the frame is.
    if let Some(picker) = deck.picker() {
        let dialog = printing_picker(commands, fonts, metrics, deck, picker, assets, cards);
        commands.entity(root).add_child(dialog);
    }
}

/// The builder's top bar: out, what is being built, and save.
fn build_bar(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let deck = state.lobby.builder();
    let bar = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap + metrics.pad),
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap),
                row_gap: px(6),
                flex_wrap: FlexWrap::Wrap,
                padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.5)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
        ))
        .id();
    let back = button(
        commands,
        fonts,
        metrics,
        if state.confirm_leave {
            "Leave without saving"
        } else {
            "‹ Decks"
        },
        Press::CloseBuilder,
        if state.confirm_leave {
            palette::DANGER
        } else {
            palette::PANEL_LIT
        },
        true,
    );
    commands.entity(bar).add_child(back);
    if metrics.frame != Frame::Phone {
        let title = commands
            .spawn((
                Text::new(if deck.editing().is_some() {
                    "Editing a deck"
                } else {
                    "A new deck"
                }),
                tf(fonts, metrics.head),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(bar).add_child(title);
    }
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    commands.entity(bar).add_child(gap);
    if !state.lobby.status().is_empty() {
        let status = commands
            .spawn((
                Text::new(state.lobby.status().to_string()),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(bar).add_child(status);
    }
    // A saved deck with nothing changed says so rather than offering a save
    // that would do nothing; a deck the gateway would refuse offers none
    // either, and the reason is standing in the problems list.
    let (label, live) = match (deck.saveable(), deck.dirty()) {
        (false, _) => ("Save deck", false),
        (true, false) => ("Saved", false),
        (true, true) => ("Save deck", !state.lobby.busy()),
    };
    let save = button(
        commands,
        fonts,
        metrics,
        label,
        Press::SaveDeck,
        palette::ACCENT,
        live,
    );
    commands.entity(bar).add_child(save);
    bar
}

/// The searchable pool: the filters, then what they leave.
#[allow(clippy::too_many_lines)] // a filter bar and a list, in order
fn pool_panel(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) -> Entity {
    let deck = state.lobby.builder();
    let panel = build_panel(commands, metrics, percent(100), 1.0);

    let search = text_box(
        commands,
        fonts,
        metrics,
        "SEARCH",
        deck.text(),
        deck.focus() == BuildField::Search,
        Press::FocusBuild(BuildField::Search),
    );
    commands.entity(panel).add_child(search);

    // A phone folds the chips away: three wrapped rows of them is most of a
    // phone screen, and what is under them is the point. Anything wider shows
    // them, because there the trade does not exist.
    let phone = metrics.frame == Frame::Phone;
    if phone {
        let bar = row(commands, metrics, true);
        let open = chip(
            commands,
            fonts,
            metrics,
            if state.filters_open {
                "Hide filters"
            } else {
                "Filters"
            },
            Press::ToggleFilters,
            state.filters_open,
        );
        commands.entity(bar).add_child(open);
        // While they are folded away, the two that are worth reaching without
        // unfolding stand out here — and "clear" only when there is something
        // to clear, because folded away is not the same as off.
        if !state.filters_open {
            if deck.filtered() {
                let clear = chip(commands, fonts, metrics, "Clear", Press::ClearFilters, true);
                commands.entity(bar).add_child(clear);
            }
            let sort = chip(
                commands,
                fonts,
                metrics,
                &format!("Sort: {}", deck.sort().label()),
                Press::CycleSort,
                false,
            );
            commands.entity(bar).add_child(sort);
        }
        commands.entity(panel).add_child(bar);
    }
    let chips_shown = !phone || state.filters_open;

    if chips_shown {
        // ---- colours
        let colors = row(commands, metrics, true);
        for (letter, name) in COLORS {
            let on = deck.colors().contains(&letter);
            let label = if metrics.frame == Frame::Desktop {
                name.to_string()
            } else {
                letter.to_string()
            };
            let c = chip(
                commands,
                fonts,
                metrics,
                &label,
                Press::ToggleColor(letter),
                on,
            );
            if on {
                commands
                    .entity(c)
                    .insert(BackgroundColor(mana_tone(letter)));
            }
            commands.entity(colors).add_child(c);
        }
        commands.entity(panel).add_child(colors);

        // ---- types
        let kinds = row(commands, metrics, true);
        for kind in KINDS {
            let on = deck.kind() == Some(kind);
            let c = chip(
                commands,
                fonts,
                metrics,
                kind,
                Press::SetKind(Some(kind)),
                on,
            );
            commands.entity(kinds).add_child(c);
        }
        commands.entity(panel).add_child(kinds);

        // ---- mana value, and the two switches
        let tail = row(commands, metrics, true);
        for cmc in 0..u32::try_from(CURVE_BUCKETS).unwrap_or(8) {
            let last = cmc as usize == CURVE_BUCKETS - 1;
            let label = if last {
                format!("{cmc}+")
            } else {
                cmc.to_string()
            };
            let c = chip(
                commands,
                fonts,
                metrics,
                &label,
                Press::SetCmc(cmc),
                deck.cmc() == Some(cmc),
            );
            commands.entity(tail).add_child(c);
        }
        commands.entity(panel).add_child(tail);

        let switches = row(commands, metrics, true);
        let sort = chip(
            commands,
            fonts,
            metrics,
            &format!("Sort: {}", deck.sort().label()),
            Press::CycleSort,
            false,
        );
        // The default is on, and it is the honest one: everything hidden by it is
        // a card the engine cannot play as printed.
        let playable = chip(
            commands,
            fonts,
            metrics,
            "Playable only",
            Press::TogglePlayable,
            deck.playable_only(),
        );
        commands.entity(switches).add_child(sort);
        commands.entity(switches).add_child(playable);
        if deck.filtered() {
            let clear = chip(
                commands,
                fonts,
                metrics,
                "Clear",
                Press::ClearFilters,
                false,
            );
            commands.entity(switches).add_child(clear);
        }
        commands.entity(panel).add_child(switches);
    }

    // ---- the results
    let shown = deck.results().len().min(SHOWN_RESULTS);
    let tally = note(
        commands,
        fonts,
        metrics,
        &if deck.loaded() {
            format!(
                "{} of {} cards{}",
                deck.results().len(),
                deck.pool().len(),
                if shown < deck.results().len() {
                    format!(" — showing {shown}, keep typing to narrow it")
                } else {
                    String::new()
                }
            )
        } else {
            "loading the card pool…".to_string()
        },
    );
    commands.entity(panel).add_child(tally);

    let list = scroller(commands, metrics, List::Pool, scrolled_to.get(List::Pool));
    commands.entity(panel).add_child(list);
    for &slot in deck.results().iter().take(shown) {
        let Some(card) = deck.card(slot) else {
            continue;
        };
        let held = deck.count_of(slot, deck.zone());
        let entry = commands
            .spawn((
                Node {
                    width: percent(100),
                    min_height: px(metrics.tap),
                    align_items: AlignItems::Center,
                    column_gap: px(metrics.gap * 0.8),
                    padding: UiRect::axes(px(metrics.pad * 0.6), px(metrics.pad * 0.3)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(if held > 0 {
                    palette::PANEL_LIT
                } else {
                    Color::NONE
                }),
                Press::AddCard(slot),
                hover_of_card(card),
            ))
            .id();
        if held > 0 {
            let badge = commands
                .spawn((
                    Text::new(format!("{held}×")),
                    tf(fonts, metrics.small),
                    TextColor(palette::ACCENT),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(entry).add_child(badge);
        }
        let name = commands
            .spawn((
                Text::new(card.name.clone()),
                tf(fonts, metrics.text),
                TextColor(if card.coverage.trustworthy() {
                    palette::INK
                } else {
                    palette::MUTED
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(entry).add_child(name);
        if metrics.frame != Frame::Phone {
            let kind = commands
                .spawn((
                    Text::new(card.type_line.clone()),
                    tf(fonts, metrics.small * 0.9),
                    TextColor(palette::MUTED),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(entry).add_child(kind);
        }
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        commands.entity(entry).add_child(gap);
        if let Some(mark) = coverage_mark(card.coverage) {
            let flag = commands
                .spawn((
                    Text::new(mark.0),
                    tf(fonts, metrics.small * 0.85),
                    TextColor(mark.1),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(entry).add_child(flag);
        }
        // A spell shows its cost as symbols; a land has none, and its power
        // and toughness is the more useful thing to put in that column.
        let cost = if card.mana_cost.is_empty() {
            let stats = card.stats.clone().unwrap_or_default();
            (!stats.is_empty()).then(|| {
                commands
                    .spawn((
                        Text::new(stats),
                        tf(fonts, metrics.small),
                        TextColor(palette::MUTED),
                        Pickable::IGNORE,
                    ))
                    .id()
            })
        } else {
            crate::manaui::spawn_cost_or_text(commands, fonts, &card.mana_cost, metrics.small)
        };
        if let Some(cost) = cost {
            commands.entity(entry).add_child(cost);
        }
        // Reading a card is its own target. There is no hover on a touch
        // screen to read one with, and the row itself has to stay the fast
        // way to add — a builder is mostly typing a name and tapping once.
        let read = chip(commands, fonts, metrics, "?", Press::Inspect(slot), false);
        commands.entity(entry).add_child(read);
        // And so is choosing which printing. Adding a card is the common
        // action and stays one tap on the row; wanting a particular piece of
        // cardboard is the rarer one and gets its own button rather than a
        // long-press nobody would find.
        let pick = chip(
            commands,
            fonts,
            metrics,
            "\u{25c8}",
            Press::PickPrint(slot),
            false,
        );
        commands.entity(entry).add_child(pick);
        commands.entity(list).add_child(entry);
    }
    if deck.loaded() && deck.results().is_empty() {
        let empty = note(
            commands,
            fonts,
            metrics,
            "nothing matches — try fewer filters",
        );
        commands.entity(list).add_child(empty);
    }
    if let Some(slot) = deck.inspecting() {
        let card = card_detail(commands, fonts, metrics, deck, slot);
        commands.entity(panel).add_child(card);
    }
    panel
}

/// The printing picker: the carousel, the language, the finish.
///
/// Drawn over the whole builder rather than beside it, on every frame size.
/// This is one question with one answer — which piece of cardboard — and a
/// panel wedged next to a card list would be the narrowest place to look at
/// art on a phone, which is the one thing this dialog exists to show.
#[allow(clippy::too_many_lines)] // one dialog, six rows, each trivial
fn printing_picker(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    deck: &DeckBuilder,
    picker: &Picker,
    assets: Option<&AssetServer>,
    cards: Option<&mut UiCards<'_>>,
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
                max_width: px(if metrics.frame == Frame::Phone {
                    520.0
                } else {
                    620.0
                }),
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap),
                padding: UiRect::all(px(metrics.pad)),
                border_radius: BorderRadius::all(px(14)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            // Swallows the tap so a press inside the dialog is not also a
            // press on the shade behind it.
            Press::PickerNothing,
        ))
        .id();
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
    let close = chip(
        commands,
        fonts,
        metrics,
        "\u{d7}",
        Press::PickerClose,
        false,
    );
    for child in [title, gap, close] {
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
    let back = chip(
        commands,
        fonts,
        metrics,
        "\u{2039}",
        Press::PickerStep(-1),
        false,
    );
    let art = picker_art(commands, fonts, metrics, picker, assets, cards);
    let forward = chip(
        commands,
        fonts,
        metrics,
        "\u{203a}",
        Press::PickerStep(1),
        false,
    );
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
        None => "no printings".to_string(),
    };
    let label = commands
        .spawn((
            Text::new(caption),
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
            "looking for other printings\u{2026}".to_string()
        } else if picker.from_catalog() {
            format!("{} of {}", picker.at() + 1, picker.len())
        } else {
            // Saying so beats implying the card was printed exactly once.
            "this gateway has no card catalog \u{2014} only this build's printing".to_string()
        },
    );
    commands.entity(panel).add_child(count);

    // ---- where in the ring, and a way to jump
    //
    // Only when there are few enough to be targets: forty dots at 44 logical
    // pixels is not a control, it is a second list.
    if picker.len() > 1 && picker.len() <= 12 {
        let dots = row(commands, metrics, true);
        for at in 0..picker.len() {
            let dot = chip(
                commands,
                fonts,
                metrics,
                if at == picker.at() {
                    "\u{25cf}"
                } else {
                    "\u{25cb}"
                },
                Press::PickerGo(at),
                at == picker.at(),
            );
            commands.entity(dots).add_child(dot);
        }
        commands.entity(panel).add_child(dots);
    }

    // ---- language
    if picker.langs().len() > 1 {
        let langs = row(commands, metrics, true);
        let all = chip(
            commands,
            fonts,
            metrics,
            "All",
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

    // ---- finish
    let finishes = row(commands, metrics, true);
    let offered = picker.finishes();
    for (finish, label) in [
        (Finish::Normal, "Plain"),
        (Finish::Foil, "Foil"),
        (Finish::Etched, "Etched"),
    ] {
        let sold = offered.contains(&finish);
        let c = chip(
            commands,
            fonts,
            metrics,
            label,
            Press::PickerFinish(finish),
            sold && picker.finish() == finish,
        );
        // A finish this printing was never sold in is shown dead rather than
        // hidden: which finishes exist is part of what a player is choosing
        // between, and a row of buttons that changes length as the carousel
        // moves is harder to read than one that greys out.
        if !sold {
            commands.entity(c).insert(Pickable::IGNORE);
            commands.entity(c).insert(BackgroundColor(Color::NONE));
        }
        commands.entity(finishes).add_child(c);
    }
    commands.entity(panel).add_child(finishes);

    // ---- the way in
    let foot = row(commands, metrics, false);
    let held = note(
        commands,
        fonts,
        metrics,
        &format!(
            "{} in the {}",
            deck.count_of(picker.slot(), picker.zone()),
            match picker.zone() {
                Zone::Main => "deck",
                Zone::Side => "sideboard",
            }
        ),
    );
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    let add = chip(commands, fonts, metrics, "Add", Press::PickerConfirm, true);
    commands.entity(add).insert(Node {
        min_height: px(metrics.tap),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.4)),
        border_radius: btn_radius(),
        ..default()
    });
    for child in [held, gap, add] {
        commands.entity(foot).add_child(child);
    }
    commands.entity(panel).add_child(foot);

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
    picker: &Picker,
    assets: Option<&AssetServer>,
    cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let height = if metrics.frame == Frame::Phone {
        240.0
    } else {
        310.0
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
            BackgroundColor(palette::PANEL_LIT),
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
    let material = match (url.as_ref(), assets, cards) {
        (Some(url), Some(assets), Some(cards)) => {
            Some(cards.preview(url, treatment(picker.finish()), assets.load(url.clone())))
        }
        _ => None,
    };
    if let Some(material) = material {
        let art = commands
            .spawn((
                MaterialNode(material),
                Node {
                    height: percent(100),
                    // A material node has no intrinsic size the way an
                    // `ImageNode` does, so without this it takes whatever
                    // width flex hands it and a card renders on its side.
                    aspect_ratio: Some(baylee_client_core::layout::CARD_ASPECT),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(holder).add_child(art);
    } else {
        let empty = note(commands, fonts, metrics, "no art for this printing");
        commands.entity(holder).add_child(empty);
    }
    holder
}

/// The picker's chosen finish as an image treatment.
pub(crate) fn treatment(finish: Finish) -> FinishTreatment {
    match finish {
        Finish::Normal => FinishTreatment::Plain,
        Finish::Foil => FinishTreatment::Foil,
        Finish::Etched => FinishTreatment::Etched,
    }
}

/// One card, read in full: what is printed on it, and what this build does
/// with it.
fn card_detail(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    deck: &DeckBuilder,
    slot: usize,
) -> Entity {
    let holder = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                padding: UiRect::all(px(metrics.pad * 0.7)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            Pickable::IGNORE,
        ))
        .id();
    let Some(card) = deck.card(slot) else {
        return holder;
    };

    let head = row(commands, metrics, false);
    let title = commands
        .spawn((
            Text::new(card.name.clone()),
            tf(fonts, metrics.text),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    let cost =
        crate::manaui::spawn_cost_or_text(commands, fonts, &card.mana_cost, metrics.small * 1.15);
    let close = chip(commands, fonts, metrics, "\u{d7}", Press::CloseCard, false);
    for child in [Some(title), Some(gap), cost, Some(close)]
        .into_iter()
        .flatten()
    {
        commands.entity(head).add_child(child);
    }
    commands.entity(holder).add_child(head);

    let kind = note(
        commands,
        fonts,
        metrics,
        &match &card.stats {
            Some(stats) => format!("{}  \u{b7}  {stats}", card.type_line),
            None => card.type_line.clone(),
        },
    );
    commands.entity(holder).add_child(kind);

    // The gateway serves rules text only when it has a catalog behind it, and
    // saying so beats an empty box that reads as a card with no abilities.
    let body = if card.oracle_text.is_empty() {
        if deck.has_text() {
            String::new()
        } else {
            "no rules text \u{2014} this gateway has no card catalog".to_string()
        }
    } else {
        card.oracle_text.clone()
    };
    if !body.is_empty() {
        let text = commands
            .spawn((
                Text::new(body),
                tf(fonts, metrics.small),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(holder).add_child(text);
    }
    if let Some(mark) = coverage_mark(card.coverage) {
        let why = match &card.note {
            Some(note) => format!("{}: {note}", mark.0),
            None => format!("{} \u{2014} this card will not play as printed", mark.0),
        };
        let line = commands
            .spawn((
                Text::new(why),
                tf(fonts, metrics.small),
                TextColor(mark.1),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(holder).add_child(line);
    }

    let menu = card_menu(commands, fonts, metrics, deck, slot);
    commands.entity(holder).add_child(menu);
    holder
}

/// What a player can do with the card they are reading.
///
/// The row itself stays the one-tap way to add to the open list, which is
/// what building a deck mostly is. Everything that needs a decision — which
/// list, which printing, whether this card leads the deck — is here, where
/// there is room to label it.
fn card_menu(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    deck: &DeckBuilder,
    slot: usize,
) -> Entity {
    let holder = row(commands, metrics, true);
    let in_main = deck.count_of(slot, Zone::Main);
    let in_side = deck.count_of(slot, Zone::Side);

    for (label, press, lit) in [
        ("+ deck", Press::AddCardTo(slot, Zone::Main), false),
        ("+ sideboard", Press::AddCardTo(slot, Zone::Side), false),
    ] {
        let button = chip(commands, fonts, metrics, label, press, lit);
        commands.entity(holder).add_child(button);
    }

    // Moving addresses a *row*, and a row index only means something in the
    // list that is open — so the move is offered on the list being shown,
    // and only when this card is actually in it.
    let open = deck.zone();
    let held = match open {
        Zone::Main => in_main,
        Zone::Side => in_side,
    };
    if held > 0
        && let Some(at) = deck.row_of(slot, open)
    {
        let label = match open {
            Zone::Main => "\u{2192} sideboard",
            Zone::Side => "\u{2192} deck",
        };
        let button = chip(commands, fonts, metrics, label, Press::MoveRow(at), false);
        commands.entity(holder).add_child(button);
        let out = chip(
            commands,
            fonts,
            metrics,
            "remove",
            Press::RemoveRow(at),
            false,
        );
        commands.entity(holder).add_child(out);
    }

    // Only cards the rules can seat get the option: the gateway refuses the
    // rest on save, and an offer that ends in a refusal is worse than none.
    if deck.card(slot).is_some_and(|card| card.commander) {
        let leading = deck.commander() == Some(slot);
        let button = chip(
            commands,
            fonts,
            metrics,
            if leading {
                "commander \u{2713}"
            } else {
                "set as commander"
            },
            if leading {
                Press::ClearCommander
            } else {
                Press::SetCommander(slot)
            },
            leading,
        );
        commands.entity(holder).add_child(button);
    }

    let where_it_is = match (in_main, in_side) {
        (0, 0) => String::new(),
        (m, 0) => format!("{m} in the deck"),
        (0, s) => format!("{s} in the sideboard"),
        (m, s) => format!("{m} in the deck, {s} in the sideboard"),
    };
    if !where_it_is.is_empty() {
        let line = note(commands, fonts, metrics, &where_it_is);
        commands.entity(holder).add_child(line);
    }
    holder
}

/// The deck itself: what it is called, what it adds up to, and every card.
#[allow(clippy::too_many_lines)] // the name, four summaries and the list
fn deck_panel(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) -> Entity {
    let deck = state.lobby.builder();
    let counts = deck.counts();
    let width = match metrics.frame {
        Frame::Phone => percent(100),
        Frame::Tablet => px(320),
        Frame::Desktop => px(380),
    };
    let grow = f32::from(u8::from(metrics.frame == Frame::Phone));
    let panel = build_panel(commands, metrics, width, grow);

    let name = text_box(
        commands,
        fonts,
        metrics,
        "DECK NAME",
        deck.name(),
        deck.focus() == BuildField::Name,
        Press::FocusBuild(BuildField::Name),
    );
    commands.entity(panel).add_child(name);

    // ---- which list is being filled
    let zones = row(commands, metrics, false);
    for (zone, label) in [
        (Zone::Main, format!("Main {}", counts.main)),
        (Zone::Side, format!("Sideboard {}", counts.side)),
    ] {
        let tab = chip(
            commands,
            fonts,
            metrics,
            &label,
            Press::SetZone(zone),
            deck.zone() == zone,
        );
        commands.entity(tab).insert(Node {
            flex_grow: 1.0,
            min_height: px(metrics.tap * 0.9),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: btn_radius(),
            ..default()
        });
        commands.entity(zones).add_child(tab);
    }
    commands.entity(zones).insert(Node {
        width: percent(100),
        column_gap: px(metrics.gap * 0.6),
        ..default()
    });
    commands.entity(panel).add_child(zones);

    let summary = note(
        commands,
        fonts,
        metrics,
        &format!(
            "{} lands · {} creatures · {} other spells",
            counts.lands, counts.creatures, counts.spells
        ),
    );
    commands.entity(panel).add_child(summary);

    let curve = curve_bars(commands, fonts, metrics, deck);
    commands.entity(panel).add_child(curve);

    let pips = pip_row(commands, fonts, metrics, deck);
    commands.entity(panel).add_child(pips);

    for problem in deck.problems() {
        let line = commands
            .spawn((
                Text::new(problem.message.clone()),
                tf(fonts, metrics.small),
                TextColor(if problem.blocking {
                    palette::DANGER
                } else {
                    palette::MUTED
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(panel).add_child(line);
    }

    // ---- the list itself
    let list = scroller(commands, metrics, List::Deck, scrolled_to.get(List::Deck));
    commands.entity(panel).add_child(list);
    let entries = deck.entries(deck.zone());
    if entries.is_empty() {
        let empty = note(
            commands,
            fonts,
            metrics,
            "empty — tap a card on the left to add it",
        );
        commands.entity(list).add_child(empty);
    }
    let mut group: Option<Group> = None;
    for (at, entry) in entries.iter().enumerate() {
        let Some(card) = deck.card(entry.slot) else {
            continue;
        };
        if group != Some(card.group()) {
            group = Some(card.group());
            let heading = commands
                .spawn((
                    Text::new(card.group().label()),
                    tf(fonts, metrics.small * 0.85),
                    TextColor(palette::MUTED),
                    Node {
                        margin: UiRect::top(px(metrics.gap * 0.6)),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(list).add_child(heading);
        }
        let row_id = commands
            .spawn((
                Node {
                    width: percent(100),
                    min_height: px(metrics.tap * 0.9),
                    align_items: AlignItems::Center,
                    column_gap: px(metrics.gap * 0.6),
                    padding: UiRect::axes(px(metrics.pad * 0.5), px(metrics.pad * 0.25)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                // Clicking a row in the deck reads the card, the same as
                // clicking one in the pool — and a row that reports nothing
                // could not be hovered for a preview either.
                Press::Inspect(entry.slot),
                hover_of_entry(card, &entry.print),
            ))
            .id();
        let count = commands
            .spawn((
                Text::new(format!("{}×", entry.count)),
                tf(fonts, metrics.small),
                TextColor(palette::ACCENT),
                Pickable::IGNORE,
            ))
            .id();
        let title = commands
            .spawn((
                Text::new(card.name.clone()),
                tf(fonts, metrics.text),
                TextColor(if card.coverage.trustworthy() {
                    palette::INK
                } else {
                    palette::MUTED
                }),
                Pickable::IGNORE,
            ))
            .id();
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        let cost =
            crate::manaui::spawn_cost_or_text(commands, fonts, &card.mana_cost, metrics.small);
        for child in [Some(count), Some(title), Some(gap), cost]
            .into_iter()
            .flatten()
        {
            commands.entity(row_id).add_child(child);
        }
        // A row that names a printing has to show it, or two lines of the
        // same card would look like a bug in the list.
        let chosen = print_mark(&entry.print);
        if !chosen.is_empty() {
            let mark = commands
                .spawn((
                    Text::new(chosen),
                    tf(fonts, metrics.small * 0.9),
                    TextColor(palette::ACCENT),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(row_id).add_child(mark);
        }
        // Two targets rather than "click removes": a deck list is read far
        // more often than it is edited, and a stray tap that silently took a
        // card out would be found much later, if at all.
        for (label, press) in [
            // Removal is by *row*, not by card: two printings of one card are
            // two lines, and a tap on one of them means that one.
            ("−", Press::RemoveRow(at)),
            ("+", Press::AddCard(entry.slot)),
            // One tap to send a copy the other way. The builder shows one
            // list at a time, so without this a card has to be removed here
            // and found again over there.
            ("\u{21c4}", Press::MoveRow(at)),
        ] {
            let step = chip(commands, fonts, metrics, label, press, false);
            commands.entity(row_id).add_child(step);
        }
        commands.entity(list).add_child(row_id);
    }

    if !entries.is_empty() {
        let clear = chip(
            commands,
            fonts,
            metrics,
            "Empty the deck",
            Press::ClearDeck,
            false,
        );
        commands.entity(panel).add_child(clear);
    }
    for missing in deck.missing() {
        let line = note(commands, fonts, metrics, &format!("dropped: {missing}"));
        commands.entity(panel).add_child(line);
    }
    panel
}

/// The mana curve, as eight bars that are also the mana-value filter.
fn curve_bars(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    deck: &DeckBuilder,
) -> Entity {
    let curve = deck.curve();
    let tallest = curve.iter().copied().max().unwrap_or(0).max(1);
    let holder = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(CURVE_HEIGHT + metrics.small * 2.4),
                align_items: AlignItems::FlexEnd,
                column_gap: px(3),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (bucket, count) in curve.iter().copied().enumerate() {
        let cmc = u32::try_from(bucket).unwrap_or(0);
        let chosen = deck.cmc() == Some(cmc);
        let column = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    row_gap: px(2),
                    ..default()
                },
                Press::SetCmc(cmc),
            ))
            .id();
        let tally = commands
            .spawn((
                Text::new(if count == 0 {
                    String::new()
                } else {
                    count.to_string()
                }),
                tf(fonts, metrics.small * 0.8),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        // A bar for an empty bucket still needs a body, or there is nothing
        // under the label to aim at.
        let height = 3.0 + (CURVE_HEIGHT - 3.0) * f32::from(count) / f32::from(tallest);
        let bar = commands
            .spawn((
                Node {
                    width: percent(100),
                    height: px(height),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(if chosen {
                    palette::ACCENT
                } else if count == 0 {
                    palette::PANEL_LIT
                } else {
                    palette::ACTIVE
                }),
                Pickable::IGNORE,
            ))
            .id();
        let label = commands
            .spawn((
                Text::new(if bucket + 1 == curve.len() {
                    format!("{cmc}+")
                } else {
                    cmc.to_string()
                }),
                tf(fonts, metrics.small * 0.8),
                TextColor(if chosen { palette::INK } else { palette::MUTED }),
                Pickable::IGNORE,
            ))
            .id();
        for child in [tally, bar, label] {
            commands.entity(column).add_child(child);
        }
        commands.entity(holder).add_child(column);
    }
    holder
}

/// The coloured pips the main deck asks for, which is what a mana base is
/// built from.
fn pip_row(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    deck: &DeckBuilder,
) -> Entity {
    let pips = deck.pips();
    let holder = commands
        .spawn((
            Node {
                width: percent(100),
                column_gap: px(metrics.gap * 0.8),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (at, count) in pips.iter().copied().enumerate() {
        if count == 0 {
            continue;
        }
        let Some(color) = baylee_core::color::Color::ALL.get(at).copied() else {
            continue;
        };
        // Symbol then count, as a decklist prints it — the letter this used
        // to show was the placeholder for exactly this.
        let pair = row(commands, metrics, false);
        let symbol = crate::manaui::spawn_pip(
            commands,
            fonts,
            baylee_client_core::manapip::of_color(color),
            metrics.small,
        );
        let text = commands
            .spawn((
                Text::new(format!(" {count}")),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(pair).add_child(symbol);
        commands.entity(pair).add_child(text);
        commands.entity(holder).add_child(pair);
    }
    holder
}

/// The colour a mana symbol is drawn in. Muted rather than saturated: these
/// sit next to body text, and a full-strength red would shout over it.
fn mana_tone(letter: char) -> Color {
    match letter {
        'W' => Color::srgb(0.93, 0.90, 0.78),
        'U' => Color::srgb(0.42, 0.65, 0.88),
        'B' => Color::srgb(0.62, 0.56, 0.68),
        'R' => Color::srgb(0.88, 0.48, 0.42),
        'G' => Color::srgb(0.46, 0.74, 0.52),
        _ => palette::MUTED,
    }
}

/// What a list says about a card the engine does not play as printed.
fn coverage_mark(coverage: Coverage) -> Option<(&'static str, Color)> {
    match coverage {
        Coverage::Implemented => None,
        Coverage::Partial => Some(("partial", palette::ACTIVE)),
        Coverage::Unimplemented => Some(("stub", palette::DANGER)),
    }
}

/// A builder panel: a column that scrolls its own contents instead of
/// growing past the bottom of the window. [`panel`] cannot: its children set
/// the height, which is right for a short list of decks and wrong for two
/// hundred cards.
fn build_panel(commands: &mut Commands, metrics: Metrics, width: Val, grow: f32) -> Entity {
    commands
        .spawn((
            Node {
                width,
                flex_grow: grow,
                flex_shrink: if grow > 0.0 { 1.0 } else { 0.0 },
                min_width: px(0),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap * 0.8),
                padding: UiRect::all(px(metrics.pad * 0.8)),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            Pickable::IGNORE,
        ))
        .id()
}
