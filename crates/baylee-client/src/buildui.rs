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
use crate::lobby::heading;
use crate::lobby::{
    FieldLook, FieldTail, Frame, List, LobbyState, Metrics, Pane, Press, Scrolled, button, chip,
    hover_of_card, hover_of_entry, music_toggle, note, print_mark, row, scroller, spacer,
    text_field,
};
use baylee_client_core::deckbuilder::{
    BuildField, CURVE_BUCKETS, Coverage, DeckBuilder, Group, Picker, Zone,
};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::images::FinishTreatment;
use baylee_core::preset::Finish;
use bevy::prelude::*;
use bevy::ui::{percent, px};

pub(crate) mod autocomplete;
mod print_picker;
use print_picker::printing_picker;
pub(crate) mod virtual_rows;

/// The tallest a mana-curve bar gets, in logical pixels.
const CURVE_HEIGHT: f32 = 54.0;

/// The colours the identity filter offers, and the pips it counts.
const COLORS: [(char, Phrase); 6] = [
    ('W', Phrase::ColorWhite),
    ('U', Phrase::ColorBlue),
    ('B', Phrase::ColorBlack),
    ('R', Phrase::ColorRed),
    ('G', Phrase::ColorGreen),
    ('C', Phrase::ColorColourless),
];

/// The card types worth a chip of their own, each with the word it is drawn
/// as. The key stays English: it is matched against a printed type line and
/// is what `Press::SetKind` carries, so translating it would filter for a
/// word no card is printed with.
const KINDS: [(&str, Phrase); 7] = [
    ("Creature", Phrase::KindCreature),
    ("Instant", Phrase::KindInstant),
    ("Sorcery", Phrase::KindSorcery),
    ("Artifact", Phrase::KindArtifact),
    ("Enchantment", Phrase::KindEnchantment),
    ("Planeswalker", Phrase::KindPlaneswalker),
    ("Land", Phrase::KindLand),
];

/// The chips narrowing the pool, in the player's own words, or `None`.
///
/// The renderer's half of [`DeckBuilder::chips_in_force`]: the model says
/// *what* filters and this says what it is called, because both words live in
/// `COLORS` and `KINDS` above — and `KINDS`' key is English on purpose, so it
/// is the only one of the four that must be translated rather than printed.
///
/// The curve bucket is two sentences and not one, because its last bucket is
/// "that or more"; `CURVE_BUCKETS` is where that is decided and this reads it
/// rather than repeating the number.
fn chips_in_words(deck: &DeckBuilder, lang: Lang) -> Option<String> {
    use baylee_client_core::deckbuilder::Chip;

    let parts: Vec<String> = deck
        .chips_in_force()
        .into_iter()
        .map(|chip| match chip {
            Chip::Colors(letters) => letters
                .iter()
                .map(|letter| {
                    COLORS
                        .iter()
                        .find(|(c, _)| c == letter)
                        .map_or_else(|| letter.to_string(), |(_, p)| p.text(lang).to_string())
                })
                .collect::<Vec<_>>()
                .join("/"),
            Chip::Kind(kind) => KINDS
                .iter()
                .find(|(k, _)| *k == kind)
                .map_or_else(|| kind.to_string(), |(_, p)| p.text(lang).to_string()),
            Chip::Cmc(cmc) => {
                let last = cmc as usize == CURVE_BUCKETS - 1;
                let phrase = if last {
                    Phrase::FilterChipCmcUp
                } else {
                    Phrase::FilterChipCmc
                };
                phrase.fill(lang, &[&cmc.to_string()])
            }
            Chip::PlayableOnly => Phrase::FilterChipPlayable.text(lang).to_string(),
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

mod commanders;
mod retained;
pub(crate) use retained::Retained;

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
) -> Retained {
    let deck = state.lobby.builder();
    let lang = state.lobby.lang();
    let phone = metrics.frame == Frame::Phone;
    let counts = deck.counts();

    let bar = build_bar(commands, state, fonts, metrics);
    commands.entity(root).add_child(bar);

    // A phone has room for one half at a time, and the switch has to say what
    // is in the other one — a deck count is the whole reason to look.
    if phone {
        let switch = row(commands, metrics, true);
        for (pane, label) in [
            (
                Pane::Cards,
                Phrase::PaneCards.fill(lang, &[&deck.results().len().to_string()]),
            ),
            (
                Pane::Deck,
                Phrase::PaneDeck.fill(lang, &[&counts.main.to_string(), &counts.side.to_string()]),
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
            flex_shrink: 0.0,
            ..default()
        });
        commands.entity(root).add_child(switch);
    }

    let body = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_basis: px(0),
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

    let mut deck_node = None;
    let mut pool_node = None;
    let mut picker_node = None;
    if !phone || state.pane == Pane::Deck {
        let list = deck_panel(commands, state, fonts, metrics, scrolled_to);
        commands.entity(body).add_child(list);
        deck_node = Some(list);
    }
    if !phone || state.pane == Pane::Cards {
        let pool = pool_panel(commands, state, fonts, metrics, scrolled_to);
        commands.entity(body).add_child(pool);
        pool_node = Some(pool);
    }

    // Last, so it sits over both halves whatever the frame is.
    if let Some(picker) = deck.picker() {
        let dialog = printing_picker(
            commands,
            fonts,
            metrics,
            lang,
            deck,
            picker,
            assets,
            cards,
            scrolled_to,
        );
        commands.entity(root).add_child(dialog);
        picker_node = Some(dialog);
    }
    Retained::new(state, root, body, bar, deck_node, pool_node, picker_node)
}

/// The builder's top bar: out, what is being built, and save.
#[allow(clippy::too_many_lines)] // editor navigation, history and save state
fn build_bar(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let deck = state.lobby.builder();
    let lang = state.lobby.lang();
    let bar = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap + metrics.pad),
                flex_shrink: 0.0,
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
            Phrase::LeaveWithoutSaving.text(lang)
        } else {
            Phrase::BackToDecks.text(lang)
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
                    Phrase::EditingADeck.text(lang)
                } else {
                    Phrase::ANewDeck.text(lang)
                }),
                tf(fonts, metrics.head),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(bar).add_child(title);
    }
    // The build, on every frame including a phone, as the lobby's bar has
    // it (#254): a deck that will not save is a bug report, and a bug report
    // without the build it came from is one nobody can act on.
    let build = commands
        .spawn((
            Text::new(baylee_build::short()),
            tf(fonts, metrics.small * 0.9),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_child(build);
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    commands.entity(bar).add_child(gap);
    // The music plays on while a deck is built (#296), so its switch is here
    // too, first on the right: nothing about the deck depends on it.
    let music = music_toggle(commands, fonts, metrics, lang);
    commands.entity(bar).add_child(music);
    let mut history_hint = None;
    {
        let history = button(
            commands,
            fonts,
            metrics,
            Phrase::DeckHistory.text(lang),
            Press::BrowseHistory,
            palette::PANEL_LIT,
            !state.lobby.busy() && state.lobby.token().is_some() && deck.editing().is_some(),
        );
        commands.entity(bar).add_child(history);
        if state.lobby.token().is_none() || deck.editing().is_none() {
            let hint = note(
                commands,
                fonts,
                metrics,
                if state.lobby.offline() {
                    Phrase::HistoryAccountHint
                } else {
                    Phrase::HistorySaveHint
                }
                .text(lang),
            );
            history_hint = Some(hint);
        }
    }
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
        (false, _) => (Phrase::SaveDeck, false),
        (true, false) => (Phrase::DeckIsSaved, false),
        (true, true) => (Phrase::SaveDeck, !state.lobby.busy()),
    };
    let save = button(
        commands,
        fonts,
        metrics,
        label.text(lang),
        Press::SaveDeck,
        palette::ACCENT,
        live,
    );
    commands.entity(bar).add_child(save);
    if let Some(hint) = history_hint {
        commands.entity(bar).add_child(hint);
    }
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
    let lang = state.lobby.lang();
    let panel = build_panel(commands, metrics, percent(100), 1.0);
    commands.entity(panel).insert(crate::lobby::dock::Dock(6));

    let search = text_field(
        commands,
        fonts,
        metrics,
        Phrase::Search.text(lang),
        &FieldLook {
            buffer: deck.buffer(BuildField::Search),
            // Not while the builder is open. The two are editors of one
            // string and only one may show a caret — a box the player cannot
            // type into while a bar blinks in it is the worse half of that.
            focused: deck.focus() == BuildField::Search
                && deck.panel().is_none()
                && deck.picker().is_none(),
            mask: None,
            press: Press::FocusBuild(BuildField::Search),
            lead: Some(crate::hud::glyph::MAGNIFIER),
            hint: Some(Phrase::SearchCards.text(lang)),
            // The cogwheel is *inside* the box, which is what says it is
            // about what the box holds rather than about the panel round it.
            // It stays lit while the builder is open, because the builder has
            // no frame of its own to say so: it is a mode of this field.
            tail: Some(FieldTail {
                glyph: crate::hud::glyph::GEAR,
                press: Press::ToggleFilterPanel,
                lit: deck.panel().is_some(),
            }),
        },
    );
    commands.entity(panel).add_child(search);
    autocomplete::draw(commands, state, fonts, metrics, search);
    // And under the box, the rows it was taken apart into. Under and not
    // over: it is what the field above it holds, so a panel floating over the
    // pool would be a second window rather than a way of writing the first.
    if let Some(built) = deck.panel() {
        let also = chips_in_words(deck, lang);
        let rows = crate::filterui::build(
            commands,
            fonts,
            built,
            baylee_client_core::cardquery::Surface::POOL,
            also.as_deref(),
            lang,
            crate::filterui::Register::LOBBY,
        );
        commands.entity(panel).add_child(rows);
    }

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
                Phrase::HideFilters.text(lang)
            } else {
                Phrase::ShowFilters.text(lang)
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
                let clear = chip(
                    commands,
                    fonts,
                    metrics,
                    Phrase::ClearFilters.text(lang),
                    Press::ClearFilters,
                    true,
                );
                commands.entity(bar).add_child(clear);
            }
            let sort = chip(
                commands,
                fonts,
                metrics,
                &Phrase::SortBy.fill(lang, &[deck.sort().label().text(lang)]),
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
                name.text(lang).to_string()
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
        for (kind, name) in KINDS {
            let on = deck.kind() == Some(kind);
            let c = chip(
                commands,
                fonts,
                metrics,
                name.text(lang),
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
            &Phrase::SortBy.fill(lang, &[deck.sort().label().text(lang)]),
            Press::CycleSort,
            false,
        );
        // The default is on, and it is the honest one: everything hidden by it is
        // a card the engine cannot play as printed.
        let playable = chip(
            commands,
            fonts,
            metrics,
            Phrase::PlayableOnly.text(lang),
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
                Phrase::ClearFilters.text(lang),
                Press::ClearFilters,
                false,
            );
            commands.entity(switches).add_child(clear);
        }
        commands.entity(panel).add_child(switches);
    }

    // ---- the results
    if let Some(partner) = state.commander_pick {
        let hint = note(
            commands,
            fonts,
            metrics,
            if partner {
                Phrase::PartnerHint
            } else {
                Phrase::CommanderHint
            }
            .text(lang),
        );
        let done = chip(
            commands,
            fonts,
            metrics,
            Phrase::DoneChoosing.text(lang),
            Press::CancelCommanderPick,
            false,
        );
        commands.entity(panel).add_children(&[hint, done]);
    }
    let results: Vec<_> = deck
        .results()
        .iter()
        .copied()
        .filter(|slot| match state.commander_pick {
            Some(true) => deck.can_partner(*slot),
            Some(false) => deck.card(*slot).is_some_and(|card| card.commander),
            None => true,
        })
        .collect();
    let shown = results.len();
    let tally = note(
        commands,
        fonts,
        metrics,
        &if deck.loaded() {
            Phrase::PoolTally.fill(
                lang,
                &[
                    &deck.results().len().to_string(),
                    &deck.pool().len().to_string(),
                    &if shown < deck.results().len() {
                        Phrase::PoolNarrow.fill(lang, &[&shown.to_string()])
                    } else {
                        String::new()
                    },
                ],
            )
        } else {
            Phrase::LoadingPool.text(lang).to_string()
        },
    );
    commands.entity(panel).add_child(tally);

    let list = scroller(commands, metrics, List::Pool, scrolled_to.get(List::Pool));
    crate::lobby::scrollbars::attach(commands, panel, list, metrics);
    if !results.is_empty() {
        let content = virtual_rows::pool(commands, state, fonts, metrics, results.clone());
        commands.entity(list).add_child(content);
    }
    if deck.loaded() && results.is_empty() {
        let empty = note(commands, fonts, metrics, Phrase::NothingMatches.text(lang));
        commands.entity(list).add_child(empty);
    }
    if let Some(slot) = deck.inspecting() {
        let card = card_detail(commands, fonts, metrics, lang, deck, slot);
        commands.entity(panel).add_child(card);
    }
    panel
}

/// The picker's chosen finish as an image treatment.
pub(crate) fn treatment(finish: Finish) -> FinishTreatment {
    match finish {
        Finish::Normal => FinishTreatment::Plain,
        Finish::Foil => FinishTreatment::Foil,
        Finish::Etched => FinishTreatment::Etched,
        Finish::Holographic => FinishTreatment::Holographic,
        Finish::Glitter => FinishTreatment::Glitter,
        Finish::Galaxy => FinishTreatment::Galaxy,
    }
}

/// One card, read in full: what is printed on it, and what this build does
/// with it.
fn card_detail(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
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
            Phrase::NoRulesText.text(lang).to_string()
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
        let mark_text = mark.0.text(lang);
        let why = match &card.note {
            Some(note) => format!("{mark_text}: {note}"),
            None => Phrase::NotAsPrinted.fill(lang, &[mark_text]),
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

    let menu = card_menu(commands, fonts, metrics, lang, deck, slot);
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
    lang: Lang,
    deck: &DeckBuilder,
    slot: usize,
) -> Entity {
    let holder = row(commands, metrics, true);
    let in_main = deck.count_of(slot, Zone::Main);
    let in_side = deck.count_of(slot, Zone::Side);

    for (label, press, lit) in [
        (Phrase::AddToDeck, Press::AddCardTo(slot, Zone::Main), false),
        (
            Phrase::AddToSideboard,
            Press::AddCardTo(slot, Zone::Side),
            false,
        ),
    ] {
        let button = chip(commands, fonts, metrics, label.text(lang), press, lit);
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
            Zone::Main => Phrase::MoveToSideboard,
            Zone::Side => Phrase::MoveToDeck,
        };
        let button = chip(
            commands,
            fonts,
            metrics,
            label.text(lang),
            Press::MoveRow(at),
            false,
        );
        commands.entity(holder).add_child(button);
        let out = chip(
            commands,
            fonts,
            metrics,
            Phrase::RemoveCard.text(lang),
            Press::RemoveRow(at),
            false,
        );
        commands.entity(holder).add_child(out);
    }

    // Only cards the rules can seat get the option: the gateway refuses the
    // rest on save, and an offer that ends in a refusal is worse than none.
    if deck.card(slot).is_some_and(|card| card.commander) {
        let leading = deck.is_commander(slot);
        let button = chip(
            commands,
            fonts,
            metrics,
            if leading {
                Phrase::IsCommander.text(lang)
            } else {
                Phrase::SetCommander.text(lang)
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
        (m, 0) => Phrase::HeldInDeck.fill(lang, &[&m.to_string()]),
        (0, s) => Phrase::HeldInSideboard.fill(lang, &[&s.to_string()]),
        (m, s) => Phrase::HeldInBoth.fill(lang, &[&m.to_string(), &s.to_string()]),
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
    let lang = state.lobby.lang();
    let counts = deck.counts();
    let stats = deck.statistics();
    let width = match metrics.frame {
        Frame::Phone => percent(100),
        Frame::Tablet => percent(54),
        Frame::Desktop => percent(52),
    };
    let grow = f32::from(u8::from(metrics.frame == Frame::Phone));
    let panel = build_panel(commands, metrics, width, grow);
    commands.entity(panel).insert(crate::lobby::dock::Dock(5));

    let overview = heading(commands, fonts, metrics, Phrase::Composition.text(lang));
    let title = row(commands, metrics, false);
    commands
        .entity(overview)
        .entry::<Node>()
        .and_modify(|mut n| {
            n.width = Val::Auto;
            n.flex_grow = 1.0;
        });
    let menu = card_action(commands, fonts, metrics, "⋯", Press::ToggleDeckActions);
    commands.entity(title).add_children(&[overview, menu]);
    commands.entity(panel).add_child(title);
    if state.deck_actions_open
        && (!deck.entries(Zone::Main).is_empty() || !deck.entries(Zone::Side).is_empty())
    {
        let clear = card_action(
            commands,
            fonts,
            metrics,
            Phrase::EmptyTheDeck.text(lang),
            Press::ClearDeck,
        );
        commands.entity(panel).add_child(clear);
    }
    let name = text_field(
        commands,
        fonts,
        metrics,
        Phrase::DeckNameLabel.text(lang),
        &FieldLook {
            buffer: deck.buffer(BuildField::Name),
            focused: deck.focus() == BuildField::Name,
            mask: None,
            press: Press::FocusBuild(BuildField::Name),
            lead: None,
            hint: None,
            tail: None,
        },
    );
    commands.entity(panel).add_child(name);
    let leaders = commanders::draw(commands, fonts, metrics, lang, deck);
    commands.entity(panel).add_child(leaders);

    // ---- which list is being filled
    let zones = row(commands, metrics, false);
    for (zone, label) in [
        (
            Zone::Main,
            Phrase::TabMain.fill(lang, &[&counts.main.to_string()]),
        ),
        (
            Zone::Side,
            Phrase::TabSide.fill(lang, &[&counts.side.to_string()]),
        ),
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
            min_height: px(metrics.tap),
            flex_shrink: 0.0,
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
        &Phrase::DeckMakeup.fill(
            lang,
            &[
                &counts.lands.to_string(),
                &counts.creatures.to_string(),
                &counts.spells.to_string(),
            ],
        ),
    );
    commands.entity(panel).add_child(summary);

    let toggle = chip(
        commands,
        fonts,
        metrics,
        Phrase::DeckStatistics.text(lang),
        Press::ToggleStatistics,
        state.stats_open,
    );
    commands.entity(panel).add_child(toggle);
    if state.stats_open {
        let statistics = row(commands, metrics, true);
        for (label, value) in [
            (Phrase::UniqueCards, stats.unique.to_string()),
            (
                Phrase::AverageMana,
                stats
                    .average_mana
                    .map_or_else(|| "—".into(), |v| format!("{v:.2}")),
            ),
            (
                Phrase::LandShare,
                stats
                    .land_share
                    .map_or_else(|| "—".into(), |v| format!("{:.0}%", v * 100.0)),
            ),
            (
                Phrase::OpeningLand,
                stats
                    .opening_land
                    .map_or_else(|| "—".into(), |v| format!("{:.1}%", v * 100.0)),
            ),
        ] {
            let stat = commands
                .spawn((
                    Node {
                        flex_grow: 1.0,
                        min_width: px(130),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(px(metrics.gap)),
                        row_gap: px(4),
                        border_radius: btn_radius(),
                        ..default()
                    },
                    BackgroundColor(palette::PANEL_LIT),
                    Pickable::IGNORE,
                ))
                .id();
            let value = heading(commands, fonts, metrics, &value);
            commands.entity(value).insert(TextColor(palette::DOCK_INK));
            let label = note(commands, fonts, metrics, label.text(lang));
            commands.entity(stat).add_children(&[value, label]);
            commands.entity(statistics).add_child(stat);
        }
        commands.entity(panel).add_child(statistics);
        let curve = curve_bars(commands, fonts, metrics, deck);
        commands.entity(panel).add_child(curve);

        let pips = pip_row(commands, fonts, metrics, deck);
        commands.entity(panel).add_child(pips);
    }

    for problem in deck.problems(lang) {
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
    crate::lobby::scrollbars::attach(commands, panel, list, metrics);
    let entries = deck.entries(deck.zone());
    if entries.is_empty() {
        let empty = note(commands, fonts, metrics, Phrase::DeckEmptyHint.text(lang));
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
                    Text::new(card.group().label().text(lang)),
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
        let row_id = virtual_rows::spawn(
            commands,
            state,
            fonts,
            metrics,
            virtual_rows::Row::Deck(at),
            at < 10,
        );
        commands.entity(list).add_child(row_id);
    }

    for missing in deck.missing() {
        let line = note(
            commands,
            fonts,
            metrics,
            &Phrase::DroppedCards.fill(lang, &[missing]),
        );
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
        commands
            .entity(pair)
            .entry::<Node>()
            .and_modify(|mut n| n.width = Val::Auto);
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
fn coverage_mark(coverage: Coverage) -> Option<(Phrase, Color)> {
    match coverage {
        Coverage::Implemented => None,
        Coverage::Partial => Some((Phrase::CoveragePartial, palette::ACTIVE)),
        Coverage::Unimplemented => Some((Phrase::CoverageStub, palette::DANGER)),
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
                padding: UiRect::all(px(metrics.pad * 1.5)),
                border_radius: BorderRadius::all(px(14)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.4)),
            Pickable::IGNORE,
        ))
        .id()
}

#[allow(clippy::too_many_lines)] // One catalog row with its explicit zone and role controls.
fn pool_row(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    slot: usize,
) -> Option<Entity> {
    let deck = state.lobby.builder();
    let lang = state.lobby.lang();
    let card = deck.card(slot)?;
    let hover = hover_of_card(card);
    let entry = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(72),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap),
                padding: UiRect::all(px(6)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            crate::ambience::Feel::tinting_to(palette::PANEL_LIT, palette::PANEL_LIT.lighter(0.06)),
            Press::Inspect(slot),
            hover.clone(),
        ))
        .id();
    let thumb = crate::lobby::thumbnails::spawn(commands, &hover);
    commands
        .entity(thumb)
        .insert((Press::PickPrint(slot), Pickable::default()));
    fill_thumbnail(commands, thumb);
    let info = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                row_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(entry).add_children(&[thumb, info]);
    let name = commands
        .spawn((
            Text::new(card.name.clone()),
            tf(fonts, metrics.text),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(name).insert(Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        ..default()
    });
    let title = row(commands, metrics, false);
    let space = commands.spawn((spacer(), Pickable::IGNORE)).id();
    commands.entity(title).add_children(&[name, space]);
    commands.entity(info).add_child(title);
    {
        let kind = commands
            .spawn((
                Text::new(card.type_line.clone()),
                tf(fonts, metrics.small * 0.9),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(info).add_child(kind);
    }

    if let Some(mark) = coverage_mark(card.coverage) {
        let flag = commands
            .spawn((
                Text::new(mark.0.text(lang)),
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
        crate::manaui::spawn_cost_or_text(commands, fonts, &card.mana_cost, metrics.small * 1.3)
    };
    if let Some(cost) = cost {
        commands.entity(title).add_child(cost);
    }
    let actions = card_actions(commands, metrics);
    for (zone, label) in [
        (Zone::Main, Phrase::LibraryMain),
        (Zone::Side, Phrase::LibrarySide),
    ] {
        if zone == Zone::Side {
            let space = commands.spawn((spacer(), Pickable::IGNORE)).id();
            commands.entity(actions).add_child(space);
        }
        let group = row(commands, metrics, false);
        commands.entity(group).entry::<Node>().and_modify(|mut n| {
            n.width = Val::Auto;
            n.column_gap = px(8);
        });
        let label = note(commands, fonts, metrics, label.text(lang));
        let less = card_action(
            commands,
            fonts,
            metrics,
            "−",
            Press::RemoveCardFrom(slot, zone),
        );
        let quantity = commands
            .spawn((
                crate::lobby::thumbnails::Quantity(slot, zone),
                Text::new(deck.count_of(slot, zone).to_string()),
                tf(fonts, metrics.small),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        let more = card_action(commands, fonts, metrics, "+", Press::AddCardTo(slot, zone));
        commands
            .entity(group)
            .add_children(&[label, less, quantity, more]);
        commands.entity(actions).add_child(group);
    }
    if card.commander && state.commander_pick.is_some() {
        let partner = state.commander_pick == Some(true);
        let leader = card_action(
            commands,
            fonts,
            metrics,
            if partner {
                Phrase::ChoosePartner
            } else {
                Phrase::SetCommander
            }
            .text(lang),
            if partner {
                Press::AddPartner(slot)
            } else {
                Press::SetCommander(slot)
            },
        );
        commands.entity(info).add_child(leader);
    }
    commands.entity(info).add_child(actions);
    Some(entry)
}

#[allow(clippy::too_many_lines)] // One deck row, including its printing and quantity controls.
fn deck_row(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    at: usize,
) -> Option<Entity> {
    let deck = state.lobby.builder();
    let lang = state.lobby.lang();
    let entry = deck.entries(deck.zone()).get(at)?;
    let card = deck.card(entry.slot)?;
    let row_id = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(70),
                flex_shrink: 0.0,
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
    commands
        .entity(row_id)
        .insert(crate::ambience::Feel::tinting_to(
            palette::PANEL_LIT,
            palette::PANEL_LIT.lighter(0.06),
        ));
    let thumb = crate::lobby::thumbnails::spawn(commands, &hover_of_entry(card, &entry.print));
    commands
        .entity(thumb)
        .insert((Press::PickRowPrint(at), Pickable::default()));
    fill_thumbnail(commands, thumb);
    let details = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: px(0),
                flex_basis: px(0),
                row_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title_row = row(commands, metrics, false);
    let actions = card_actions(commands, metrics);
    let kind = note(commands, fonts, metrics, &card.type_line);
    commands
        .entity(details)
        .add_children(&[title_row, kind, actions]);
    commands.entity(row_id).add_children(&[thumb, details]);
    let count = commands
        .spawn((
            Text::new(entry.count.to_string()),
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
        crate::manaui::spawn_cost_or_text(commands, fonts, &card.mana_cost, metrics.small * 1.3);
    commands.entity(title).insert(Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        ..default()
    });
    for child in [Some(title), Some(gap), cost].into_iter().flatten() {
        commands.entity(title_row).add_child(child);
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
        commands.entity(title_row).add_child(mark);
    }
    // Two targets rather than "click removes": a deck list is read far
    // more often than it is edited, and a stray tap that silently took a
    // card out would be found much later, if at all.
    for (label, press) in [
        // Removal is by *row*, not by card: two printings of one card are
        // two lines, and a tap on one of them means that one.
        ("−", Press::RemoveRow(at)),
        ("+", Press::AddRow(at)),
        // One tap to send a copy the other way. The builder shows one
        // list at a time, so without this a card has to be removed here
        // and found again over there.
        (
            if deck.zone() == Zone::Main {
                Phrase::MoveToSideboard.text(lang)
            } else {
                Phrase::MoveToDeck.text(lang)
            },
            Press::MoveRow(at),
        ),
    ] {
        if matches!(press, Press::MoveRow(_)) {
            let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
            commands.entity(actions).add_child(gap);
        }
        let step = card_action(commands, fonts, metrics, label, press);
        commands.entity(actions).add_child(step);
        if matches!(press, Press::RemoveRow(_)) {
            commands.entity(actions).add_child(count);
        }
    }
    Some(row_id)
}

fn card_actions(commands: &mut Commands, metrics: Metrics) -> Entity {
    let id = row(commands, metrics, true);
    commands.entity(id).entry::<Node>().and_modify(|mut n| {
        n.column_gap = px(10);
        n.row_gap = px(8);
    });
    id
}

fn card_action(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    press: Press,
) -> Entity {
    let id = crate::hud::answer_sized(
        commands,
        fonts,
        label,
        crate::hud::ButtonWeight::Secondary,
        None,
        if metrics.frame == Frame::Phone {
            34.0
        } else {
            26.0
        },
        metrics.small,
    );
    crate::lobby::button_style::icon(commands, fonts, id, press, metrics.small);
    commands
        .entity(id)
        .insert(press)
        .entry::<Node>()
        .and_modify(|mut n| {
            n.padding = UiRect::axes(px(9), px(2));
        });
    id
}

fn fill_thumbnail(commands: &mut Commands, thumb: Entity) {
    commands.entity(thumb).entry::<Node>().and_modify(|mut n| {
        n.width = Val::Auto;
        n.height = percent(100);
        n.aspect_ratio = Some(5.0 / 7.0);
    });
}
