//! The settings screen (`DESIGN-v5` §12, WP5): ten sections in a sidebar
//! with a search field at its top, one section's rows at a time in a panel
//! with its title, scope badge and *Reset this section*, and a save line.
//!
//! Every row says where it is kept (the storage tag): this device
//! (`ClientSettings`), your account (`Preferences`, over `/settings`), or
//! this machine (`llm-seat.json`). Before sign-in the account's rows are
//! read-only ("Sign in to change"); the screen is open from the front door
//! too. The map of sections and rows is client-core's
//! (`settings_map`): this module draws it.
//!
//! Size classes (§2.7): Wide and Vast stand the sidebar beside the panel;
//! Narrow and Compact put the search and a chip row of sections over it; a
//! phone keeps the section list, 180 wide, at the left.

pub(crate) mod bindings;
pub(crate) mod gameplay;
pub(crate) mod keys;
pub(crate) mod rows;
pub(crate) mod sections;

use baylee_client_core::graphics::Graphics;
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::prefs::Preferences;
use baylee_client_core::settings_map::{self, Builds, Scope, Section};
use bevy::prelude::*;

use crate::lobby::{List, LobbyState, Metrics, Press, Scrollable, SettingsPress};
use crate::shellkit::Frame;
use crate::shellkit::controls::{self, Kit};
use crate::shellkit::focus::Current;
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::role::Role;
use crate::shellkit::{surfaces, tokens};

use keys::NavSection;
use rows::{Out, item, stop};

/// What this build has (`settings_map::Builds`).
#[must_use]
pub(crate) const fn builds() -> Builds {
    Builds {
        desktop: crate::seatpanel::DESKTOP && cfg!(not(target_arch = "wasm32")),
        web: cfg!(target_arch = "wasm32"),
        phone: cfg!(any(target_os = "android", target_os = "ios")),
    }
}

/// What the screen is drawn from.
pub(crate) struct View<'a> {
    /// The lobby (the section, the query, the seat panel, the account).
    pub(crate) state: &'a LobbyState,
    /// The account's preferences.
    pub(crate) prefs: &'a Preferences,
    /// This device's settings.
    pub(crate) settings: Option<&'a crate::settings::ClientSettings>,
    /// The graphics shown: the device's choice, or what is in force.
    pub(crate) graphics: Graphics,
    /// The display mode's trial: seconds left.
    pub(crate) trial: Option<u32>,
    /// How many monitors the window can stand on.
    pub(crate) monitors: usize,
    /// What this build has.
    pub(crate) builds: Builds,
    /// The lobby's metrics, for the panels this screen hosts as they are
    /// (the seat panel, the updater).
    pub(crate) metrics: Metrics,
    /// Where the panel was scrolled to.
    pub(crate) scroll: f32,
    /// Where a language-model profile's sheet was scrolled to.
    pub(crate) sheet_scroll: f32,
    /// Where the section list was scrolled to.
    pub(crate) nav_scroll: f32,
}

/// The sidebar's width on a desktop and on a phone.
fn sidebar_width(kit: Kit) -> Val {
    match kit.m.frame {
        // A phone's is narrow on purpose, but its longest word has to fit
        // at the largest step (German "Sprachmodelle" at XL).
        Frame::Phone => px_fixed(180.0 * kit.m.factor.max(1.0)),
        _ => kit.m.px(260.0),
    }
}

/// Whether the sections stand in a sidebar (else a chip row over the
/// panel).
const fn sidebar(frame: Frame) -> bool {
    matches!(frame, Frame::Wide | Frame::Vast | Frame::Phone)
}

/// Draws the screen under `root`.
pub(crate) fn screen(commands: &mut Commands, root: Entity, view: &View, kit: Kit) {
    let state = view.state;
    let lang = state.lobby.lang();
    let signed_in = state.lobby.token().is_some();
    let side = sidebar(kit.m.frame);
    let body = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: px_fixed(kit.m.body_max),
                align_self: AlignSelf::Center,
                flex_grow: 1.0,
                min_height: px_fixed(0.0),
                flex_direction: if side {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                padding: UiRect::all(px_fixed(kit.m.body)),
                column_gap: px_fixed(kit.m.body),
                row_gap: px_fixed(kit.m.gap),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(body);
    let nav = nav(commands, view, kit, side);
    let panel = section_panel(commands, view, kit, signed_in, lang);
    commands.entity(body).add_children(&[nav, panel]);
    // A language-model profile's sheet, over the list it was opened from.
    if state.settings_view.profile_sheet
        && state.settings_section() == Section::LanguageModels
        && state.settings_query().is_empty()
        && let Some(sheet) = crate::seatpanel::sheet(
            commands,
            &state.seat,
            lang,
            kit.fonts,
            view.metrics,
            state.settings_view.profile_advanced,
            view.sheet_scroll,
        )
    {
        commands.entity(root).add_child(sheet);
    }
}

/// The settings search: a lobby field ([`Field::SettingsSearch`]) carrying
/// the screen's `search` stop.
fn search_field(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    use baylee_client_core::lobby::Field;
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let typed = !lobby.field(Field::SettingsSearch).is_empty();
    let look = crate::lobby::FieldLook {
        buffer: lobby.buffer(Field::SettingsSearch),
        focused: lobby.focus() == Field::SettingsSearch && lobby.typing_here(),
        mask: None,
        press: Press::Shared(crate::lobby::SharedPress::Focus(Field::SettingsSearch)),
        lead: Some(crate::hud::glyph::MAGNIFIER),
        hint: Some(Phrase::SettingsSearch.text(lang)),
        tail: typed.then_some(crate::lobby::FieldTail {
            glyph: crate::hud::glyph::CLOSE,
            press: Press::Settings(SettingsPress::ClearSearch),
            lit: false,
        }),
    };
    let field = crate::lobby::text_field_with(
        commands,
        kit.fonts,
        crate::lobby::front::faces::field_metrics(kit),
        "",
        &look,
        Some(crate::lobby::FieldStops {
            field: stop("search"),
            typed: Some(Field::SettingsSearch),
            eye: None,
        }),
    );
    commands
        .entity(field)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.width = Val::Percent(100.0);
            node.flex_shrink = 0.0;
        });
    field
}

/// The sections: a sidebar with the search at its top and the build at its
/// foot, or (Narrow, Compact) the search and a chip row.
fn nav(commands: &mut Commands, view: &View, kit: Kit, side: bool) -> Entity {
    let state = view.state;
    let lang = state.lobby.lang();
    let holder = nav_holder(commands, kit, side, view.nav_scroll);
    // The way back to where the screen was opened over (the builder, the
    // front door, Play): before sign-in no header stands over it at all.
    {
        let back = controls::button(
            commands,
            kit,
            &format!("\u{2039} {}", Phrase::Back.text(lang)),
            controls::Weight::Secondary,
            controls::Live::Yes,
            // Esc is its key only before sign-in, where no header stands
            // over the screen; signed in, Settings' Esc is nothing
            // (`KEYBOARD.md` §2.5) and the header leaves it.
            (state.lobby.token().is_none() && !state.lobby.offline()).then_some("Esc"),
            (Press::Settings(SettingsPress::CloseSettings), stop("back")),
        );
        commands.entity(holder).add_child(back);
    }
    // The lobby's own field, as the username is (owner, 09.10.2026): its
    // caret, selection, keys and clipboard, the magnifier before the text
    // and a `×` that empties it once anything is typed.
    let search = search_field(commands, state, kit);
    commands.entity(holder).add_child(search);
    let shown = state.settings_section();
    let offered: Vec<Section> = Section::ALL
        .into_iter()
        .filter(|s| s.offered(view.builds))
        .collect();
    for (i, section) in offered.iter().enumerate() {
        let on = *section == shown && state.settings_query().is_empty();
        let entry = if side {
            nav_item(commands, kit, section.name().text(lang), on)
        } else {
            controls::chip(
                commands,
                kit,
                section.short().text(lang),
                on,
                None,
                false,
                (),
            )
        };
        commands.entity(entry).insert((
            Press::Settings(SettingsPress::Section(*section)),
            item("nav", i),
            Current(on),
            NavSection(*section),
        ));
        commands.entity(holder).add_child(entry);
    }
    // The build at the sidebar's foot, where it has the height (a phone's
    // sidebar is the sections and no more).
    if side && kit.m.frame != Frame::Phone {
        let foot = commands
            .spawn((
                Text::new(baylee_build::short()),
                crate::hud::tf(kit.fonts, kit.m.small),
                TextColor(tokens::MUTED),
                Node {
                    margin: UiRect::top(Val::Auto),
                    padding: UiRect::all(kit.m.px(8.0)),
                    // Its two lines, never squeezed into one line's height
                    // (the second then lay outside its own box).
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(holder).add_child(foot);
    }
    holder
}

/// The nav's own panel: a column beside the rows, or a wrapping row.
fn nav_holder(commands: &mut Commands, kit: Kit, side: bool, scroll: f32) -> Entity {
    commands
        .spawn((
            Role::Panel,
            Node {
                width: if side {
                    sidebar_width(kit)
                } else {
                    Val::Percent(100.0)
                },
                flex_shrink: 0.0,
                flex_direction: if side {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                flex_wrap: if side {
                    FlexWrap::NoWrap
                } else {
                    FlexWrap::Wrap
                },
                align_items: if side {
                    AlignItems::Stretch
                } else {
                    AlignItems::Center
                },
                padding: UiRect::all(px_fixed(kit.m.pad * 0.7)),
                row_gap: kit.m.px(4.0),
                column_gap: kit.m.px(6.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
            // A phone's list is taller than its window: it scrolls under
            // a finger or a wheel, and keeps its place across rebuilds.
            ScrollPosition(Vec2::new(0.0, scroll)),
            Scrollable(List::SettingsNav),
            Pickable::default(),
        ))
        .id()
}

/// One section in the sidebar: a full-width row, lit when shown.
fn nav_item(commands: &mut Commands, kit: Kit, text: &str, on: bool) -> Entity {
    let face = commands
        .spawn((
            Role::MenuItem,
            Node {
                min_height: px_fixed(kit.m.control),
                width: Val::Percent(100.0),
                padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                align_items: AlignItems::Center,
                border: UiRect::left(px_fixed(3.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(if on { tokens::SELECTED } else { Color::NONE }),
            // The gold bar at its left edge alone.
            BorderColor {
                left: if on { tokens::GOLD } else { Color::NONE },
                ..BorderColor::all(Color::NONE)
            },
        ))
        .id();
    let words = controls::label(
        commands,
        kit,
        text,
        kit.m.text,
        if on { tokens::INK } else { tokens::MUTED },
    );
    commands
        .entity(words)
        .insert(TextLayout::new(Justify::Left, LineBreak::WordBoundary));
    commands.entity(face).add_child(words);
    let wrapper = controls::hit(commands, kit, face, ());
    commands
        .entity(wrapper)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.width = Val::Percent(100.0);
            node.justify_content = JustifyContent::FlexStart;
        });
    // A line of writing: the pointer lights it and never grows it (a
    // grown row stands outside its own box), and it rests at its own
    // ground — the lit one's included, which a `Feel` resting at nothing
    // painted out.
    let ground = if on { tokens::SELECTED } else { Color::NONE };
    commands.entity(face).insert((
        Pickable::default(),
        crate::ambience::Feel::tinting_to(ground, tokens::SELECTED),
    ));
    wrapper
}

/// Whether a section has rows a reset puts back.
const fn resettable(section: Section) -> bool {
    matches!(
        section,
        Section::Graphics
            | Section::Audio
            | Section::Display
            | Section::Controls
            | Section::Gameplay
    )
}

/// The panel: the section's head and rows, or the search's results.
fn section_panel(
    commands: &mut Commands,
    view: &View,
    kit: Kit,
    signed_in: bool,
    lang: Lang,
) -> Entity {
    let state = view.state;
    let panel = commands
        .spawn((
            Role::Panel,
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                min_height: px_fixed(0.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: px_fixed(kit.m.gap * 0.5),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    let query = state.settings_query();
    let section = state.settings_section();
    let head = panel_head(commands, view, kit, lang);
    let rows = commands
        .spawn((
            Role::Scroll,
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_height: px_fixed(0.0),
                row_gap: px_fixed(kit.m.gap * 0.5),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition(Vec2::new(0.0, view.scroll)),
            Scrollable(List::Settings),
            Pickable::default(),
        ))
        .id();
    let mut out = Out {
        commands,
        kit,
        lang,
        signed_in,
        column: rows,
    };
    if query.is_empty() {
        sections::draw(section, &mut out, view);
    } else {
        results(&mut out, view, query);
    }
    let save = commands
        .spawn((
            Text::new(if signed_in {
                Phrase::SettingsSaveLine.text(lang)
            } else {
                Phrase::SettingsSaveLineOffline.text(lang)
            }),
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Node {
                flex_shrink: 0.0,
                padding: UiRect::top(kit.m.px(8.0)),
                border: UiRect::top(px_fixed(1.0)),
                ..default()
            },
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    // A phone's height is the scarce axis: the line scrolls with the rows
    // there, after the last, so three rows stand in the 390 px (§12, M4-6).
    if kit.m.frame == Frame::Phone {
        commands.entity(rows).add_child(save);
        commands.entity(panel).add_children(&[head, rows]);
    } else {
        commands.entity(panel).add_children(&[head, rows, save]);
    }
    panel
}

/// The panel's head: the section's title, its scope and Reset this
/// section, or the search's title.
fn panel_head(commands: &mut Commands, view: &View, kit: Kit, lang: Lang) -> Entity {
    let query = view.state.settings_query();
    let section = view.state.settings_section();
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                column_gap: kit.m.px(12.0),
                flex_shrink: 0.0,
                flex_wrap: FlexWrap::Wrap,
                padding: UiRect::bottom(kit.m.px(8.0)),
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(if query.is_empty() {
                section.name().text(lang).to_string()
            } else {
                Phrase::SettingsSearch
                    .text(lang)
                    .trim_end_matches('\u{2026}')
                    .to_string()
            }),
            crate::hud::tf(kit.fonts, kit.m.h1),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(head).add_child(title);
    if query.is_empty() {
        let badge = scope_badge(commands, kit, section.scope(), lang);
        commands.entity(head).add_child(badge);
        let gap = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(head).add_child(gap);
        if resettable(section) {
            let reset = controls::button(
                commands,
                kit,
                Phrase::SettingsResetSection.text(lang),
                controls::Weight::Ghost,
                controls::Live::Yes,
                None,
                (
                    Press::Settings(SettingsPress::ResetSection(section)),
                    stop("reset"),
                ),
            );
            commands.entity(head).add_child(reset);
        }
    }
    head
}

/// The scope badge beside a section's title.
fn scope_badge(commands: &mut Commands, kit: Kit, scope: Scope, lang: Lang) -> Entity {
    let badge = commands
        .spawn((
            Role::Tag,
            Node {
                padding: UiRect::axes(kit.m.px(8.0), kit.m.px(2.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(match scope {
                Scope::Account => tokens::ACCENT,
                Scope::Device | Scope::Machine => tokens::BORDER,
            }),
            Pickable::IGNORE,
        ))
        .id();
    let words = controls::label(
        commands,
        kit,
        scope.tag().text(lang),
        kit.m.small,
        tokens::MUTED,
    );
    commands.entity(badge).add_child(words);
    badge
}

/// The search's results: each row with its section as a crumb; a press
/// shows that section.
fn results(out: &mut Out, view: &View, query: &str) {
    let lang = out.lang;
    let found = settings_map::search(query, lang, view.builds, view.monitors);
    if found.is_empty() {
        let none = out.words(Phrase::SettingsNoMatch.text(lang), true);
        out.commands.entity(out.column).add_child(none);
        return;
    }
    for (i, row) in found.into_iter().enumerate() {
        let def = settings_map::of(row);
        let crumb = format!(
            "{} \u{b7} {}",
            def.section.name().text(lang),
            def.help.text(lang)
        );
        let line = surfaces::list_row(
            out.commands,
            out.kit,
            def.label.text(lang),
            &crumb,
            &[],
            (Press::Settings(SettingsPress::Jump(row)), item("result", i)),
        );
        out.commands.entity(out.column).add_child(line);
    }
}

/// The section a press of the search's results or the nav shows, and the
/// query cleared.
pub(crate) fn show(state: &mut LobbyState, section: Section) {
    if state.settings_section() != section {
        state.set_settings_section(section);
    }
    if state.settings_view.profile_sheet {
        state.settings_view.profile_sheet = false;
    }
    if !state.settings_query().is_empty() {
        state.set_settings_query(String::new());
    }
}

/// Puts a section's rows back to their defaults (the account's included:
/// the section is one thing to the player, wherever its rows are kept).
pub(crate) fn reset_section(
    section: Section,
    prefs: &mut crate::prefs::Prefs,
    settings: Option<&mut crate::settings::ClientSettings>,
) {
    let defaults = Preferences::default();
    match section {
        Section::Graphics => {
            if let Some(settings) = settings {
                settings.graphics = None;
                settings.table = baylee_client_core::tableview::TableView::default();
                settings.save();
            }
            let mut edit = prefs.edit();
            edit.atmosphere = defaults.atmosphere;
            edit.reduce_motion = defaults.reduce_motion;
            edit.sky = defaults.sky;
        }
        Section::Audio => {
            if let Some(settings) = settings {
                settings.audio = baylee_client_core::audiomix::AudioMix::default();
                settings.music = baylee_client_core::music::MusicLevel::default();
                settings.save();
            }
            prefs.edit().sound = defaults.sound;
        }
        Section::Display => {
            if let Some(settings) = settings {
                settings.text_size = crate::shellkit::TextSize::default();
                settings.preview_scale = 1.0;
                settings.prefer_text_view = false;
                settings.save();
            }
        }
        Section::Controls => {
            let mut edit = prefs.edit();
            edit.keymap = defaults.keymap;
            edit.shell_keys = defaults.shell_keys;
        }
        Section::Gameplay => {
            let mut edit = prefs.edit();
            edit.auto = defaults.auto;
            edit.orders = defaults.orders;
            edit.ability_orders.clear();
        }
        Section::Account
        | Section::Network
        | Section::LanguageModels
        | Section::Updates
        | Section::Privacy => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::automation::RAIL_ROWS;
    use baylee_client_core::prefs::{Action, AutoRule};

    /// One entry per language, each naming itself.
    #[test]
    fn every_language_names_itself_and_is_offered() {
        let mut codes: Vec<&str> = Lang::ALL.iter().map(|l| l.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), Lang::ALL.len(), "two languages share a code");
        for lang in Lang::ALL {
            assert!(!lang.name().is_empty());
        }
    }

    /// The screen lists one row per action and one switch per rule, from
    /// the `ALL` constants: an action missing there is one nobody can bind.
    #[test]
    fn every_action_and_every_rule_has_a_row() {
        assert_eq!(Action::ALL.len(), 39);
        assert_eq!(AutoRule::ALL.len(), 4);
        for action in Action::ALL {
            assert!(!action.label().text(Lang::En).is_empty());
            assert!(!action.group().text(Lang::En).is_empty());
        }
        for rule in AutoRule::ALL {
            assert!(!rule.label().text(Lang::En).is_empty());
            assert!(!rule.detail().text(Lang::En).is_empty());
        }
        for step in RAIL_ROWS {
            assert!(!step.name().text(Lang::En).is_empty());
        }
    }

    /// Actions are drawn under group headings, which change only when the
    /// group does: `ALL` keeps each group's actions together.
    #[test]
    fn the_action_list_keeps_each_group_in_one_run() {
        let mut seen: Vec<Phrase> = Vec::new();
        for action in Action::ALL {
            if seen.last() != Some(&action.group()) {
                assert!(!seen.contains(&action.group()));
                seen.push(action.group());
            }
        }
    }

    /// This build's map: a desktop test build has the updater, a browser
    /// build would not (`settings_map` tests the rule).
    #[test]
    fn this_build_says_what_it_has() {
        let here = builds();
        assert_eq!(here.web, cfg!(target_arch = "wasm32"));
        assert_eq!(
            Section::Updates.offered(here),
            !cfg!(any(
                target_arch = "wasm32",
                target_os = "android",
                target_os = "ios"
            ))
        );
    }
}
