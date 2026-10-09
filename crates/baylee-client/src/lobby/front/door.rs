//! The front door's frame around the card (`DESIGN-v5` §3, §2.7): the
//! logo, the tagline, the text row, the colophon and the About sheet — and
//! where each stands in each size class.
//!
//! Principle 8: the notices travel with the door. Every face at every size
//! class carries a colophon on a mist plate: on Wide and Vast the full text
//! (the build, the Fan Content Policy's notice word for word, Scryfall's
//! credit, the source link) and the QR code; on the smaller classes the
//! one-line colophon, whose notice opens the About sheet with the full
//! text one press away (`docs/legal.md` §"The front door's notices").
//! No text stands on the painting without a mist plate (§2.2).

#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::super::*;
use super::keys::{self, ABOUT};
use super::{FAN_CONTENT_NOTICE, FrontPress, source_address};
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::focus::{Current, Stop};
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::role::Role;
use crate::shellkit::{surfaces, tokens};

/// Whether the full colophon (and the QR) stands under the card: on Wide
/// and Vast, where the window is tall enough to hold it beside the card
/// and the text row (an 820-px tablet takes the one-line colophon and
/// About, as the smaller classes do).
pub(super) fn full_colophon(kit: Kit, height: f32) -> bool {
    matches!(kit.m.frame, Frame::Wide | Frame::Vast) && height >= FULL_COLOPHON_HEIGHT
}

/// The least window height (logical px) the full colophon stands in.
pub(in crate::lobby) const FULL_COLOPHON_HEIGHT: f32 = 900.0;

/// A mist plate holding a row of things that would stand on the painting.
fn plate(commands: &mut Commands, kit: Kit) -> Entity {
    commands
        .spawn((
            Role::Mist,
            Node {
                padding: UiRect::axes(kit.m.px(10.0), kit.m.px(2.0)),
                column_gap: kit.m.px(2.0),
                row_gap: kit.m.px(2.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_wrap: FlexWrap::Wrap,
                max_width: Val::Percent(100.0),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::MIST),
            Pickable::IGNORE,
        ))
        .id()
}

/// How a [`link`] reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    /// A plain word.
    Plain,
    /// The chosen one of a group (the language): on a selected ground.
    Current,
    /// An address or a door to more text: underlined.
    Underlined,
}

/// A word on a mist plate a press reaches: a ghost button in the kit's hit
/// area.
fn link(commands: &mut Commands, kit: Kit, text: &str, look: Look, action: impl Bundle) -> Entity {
    // A link, not a button: it reads as words (an address may be long), so
    // it holds no button's label budget; its hit area is the kit's.
    let face = commands
        .spawn((
            Node {
                // Room either side of the word, so the chosen language's
                // ground does not end at its letters (owner, 09.10.2026).
                padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                min_height: px_fixed(kit.m.control * 0.8),
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(if look == Look::Current {
                tokens::SELECTED
            } else {
                Color::NONE
            }),
        ))
        .id();
    let mut words = commands.spawn((
        Text::new(text),
        crate::hud::tf(kit.fonts, kit.m.small),
        TextColor(tokens::INK),
        TextLayout::no_wrap(),
        Pickable::IGNORE,
    ));
    if look == Look::Underlined {
        words.insert((Underline, UnderlineColor(tokens::INK)));
    }
    let words = words.id();
    commands.entity(face).add_child(words);
    let wrapper = controls::hit(commands, kit, face, action);
    // A word lights under the pointer as a ghost button does (the face is
    // its hit area's: `ambience::feel` reads the wrapper).
    commands.entity(face).insert(if look == Look::Current {
        crate::ambience::Feel::new(tokens::SELECTED)
    } else {
        crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER)
    });
    wrapper
}

/// A separator dot between links.
fn sep(commands: &mut Commands, kit: Kit) -> Entity {
    controls::label(commands, kit, "\u{b7}", kit.m.small, tokens::INK)
}

/// The text row on its mist plate: the languages (one radio group), the
/// music, Settings, Play offline, About (A6: what the gear used to hide).
/// On a phone it stands as a column in the pane beside the card, where the
/// width is (§2.7: height is the scarce axis).
pub(super) fn text_row(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    music_on: bool,
) -> Entity {
    let lang = state.lobby.lang();
    let column = kit.m.frame == Frame::Phone;
    let row = plate(commands, kit);
    if column {
        let pad = UiRect::all(kit.m.px(2.0));
        commands
            .entity(row)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.flex_direction = FlexDirection::Column;
                node.align_items = AlignItems::Stretch;
                node.padding = pad;
            });
    }
    let table = keys::table_of(state).name;
    let mut items: Vec<Entity> = Vec::new();
    for (i, offered) in Lang::ALL.into_iter().enumerate() {
        let on = offered == lang;
        let item = link(
            commands,
            kit,
            offered.name(),
            if on { Look::Current } else { Look::Plain },
            (
                Press::Shared(SharedPress::PickLang(offered)),
                Stop::item(table, "lang", u8::try_from(i).unwrap_or(u8::MAX)),
                Current(on),
            ),
        );
        items.push(item);
    }
    // On a phone's column the two languages share one line.
    if column {
        // Side by side where the column has the room, one over the other
        // at the larger steps.
        let pair = commands
            .spawn((
                Node {
                    justify_content: JustifyContent::Center,
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(pair).add_children(&items);
        items = vec![pair];
    }
    let rest: [(&str, Press, &'static str); 4] = [
        (
            if music_on {
                Phrase::MusicPlaying.text(lang)
            } else {
                Phrase::MusicSilent.text(lang)
            },
            Press::Shared(SharedPress::ToggleMusic),
            "music",
        ),
        (
            Phrase::Settings.text(lang),
            Press::Settings(SettingsPress::OpenSettings),
            "settings",
        ),
        (
            Phrase::PlayOffline.text(lang),
            Press::Front(FrontPress::PlayOffline),
            "offline",
        ),
        (
            Phrase::FrontAbout.text(lang),
            Press::Front(FrontPress::About(true)),
            "about",
        ),
    ];
    for (text, press, id) in rest {
        if !column {
            items.push(sep(commands, kit));
        }
        let item = link(
            commands,
            kit,
            text,
            Look::Plain,
            (press, Stop::new(table, id)),
        );
        items.push(item);
    }
    commands.entity(row).add_children(&items);
    row
}

/// The one-line colophon (§3), on every face below Wide: the notice in the
/// policy's own short words (its full text one press away, on About),
/// Scryfall's credit, and the source (the address itself on About, with its
/// code). One line at 640 px. The build stands under the text row
/// ([`version_line`], owner 09.10.2026).
pub(super) fn one_line(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let table = keys::table_of(state).name;
    let row = plate(commands, kit);
    let notice = link(
        commands,
        kit,
        Phrase::ColophonNotice.text(lang),
        Look::Underlined,
        (
            Press::Front(FrontPress::About(true)),
            Stop::new(table, "notice"),
        ),
    );
    let credit = controls::label(
        commands,
        kit,
        Phrase::ColophonScryfall.text(lang),
        kit.m.small,
        tokens::INK,
    );
    // The link opens the address the gateway gave only where it passed the
    // check at the door; else it opens About, which says it as text.
    let source = link(
        commands,
        kit,
        Phrase::ColophonSource.text(lang),
        Look::Underlined,
        (
            Press::Front(if state.source_code.is_some() {
                FrontPress::OpenSource
            } else {
                FrontPress::About(true)
            }),
            Stop::new(table, "source"),
        ),
    );
    let parts = [notice, credit, source];
    for (i, part) in parts.into_iter().enumerate() {
        if i > 0 {
            let s = sep(commands, kit);
            commands.entity(row).add_child(s);
        }
        commands.entity(row).add_child(part);
    }
    row
}

/// The full colophon (Wide, Vast): the Fan Content notice word for word and
/// Scryfall's credit, set as a short centred paragraph on a mist plate
/// rather than run into one long line (owner, 09.10.2026). The build stands
/// under the text row ([`version_line`]) and the source with its QR in the
/// top corner ([`source_corner`]).
pub(super) fn full(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let holder = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(kit.m.px(8.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Role::Mist,
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                // About two lines of the notice: a paragraph, not a ribbon
                // across the window. A definite width, so the notice is
                // measured wrapped and the plate holds all its lines.
                width: Val::Percent(100.0),
                max_width: kit.m.px(COLOPHON_MEASURE),
                padding: UiRect::axes(kit.m.px(16.0), kit.m.px(8.0)),
                row_gap: kit.m.px(4.0),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::MIST),
            Pickable::IGNORE,
        ))
        .id();
    let line = |commands: &mut Commands, text: &str| {
        commands
            .spawn((
                Text::new(text),
                crate::hud::tf(kit.fonts, kit.m.small),
                TextColor(tokens::INK),
                TextLayout::justify(Justify::Center),
                Pickable::IGNORE,
            ))
            .id()
    };
    let notice = line(commands, FAN_CONTENT_NOTICE);
    let credit = line(commands, Phrase::ScryfallCredit.text(lang));
    commands.entity(words).add_children(&[notice, credit]);
    commands.entity(holder).add_child(words);
    holder
}

/// How wide the full colophon's paragraph runs at the default step: the
/// notice in two lines at 1920 px.
const COLOPHON_MEASURE: f32 = 760.0;

/// The source offer in the window's top corner on Wide and Vast (owner,
/// 09.10.2026; `docs/legal.md` §"The front door's notices"): its QR, and
/// under it "Source code (AGPL-3.0): <address>" as text and link, on a
/// mist plate. The AGPL §13
/// offer stays on screen; it no longer stretches the colophon.
pub(super) fn source_corner(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let table = keys::table_of(state).name;
    let corner = commands
        .spawn((
            Role::Mist,
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(kit.m.body),
                right: px_fixed(kit.m.body),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                max_width: kit.m.px(SOURCE_MEASURE),
                padding: UiRect::all(kit.m.px(8.0)),
                row_gap: kit.m.px(4.0),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::MIST),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(code) = state.source_code.as_ref() {
        let picture = qr(commands, code);
        commands.entity(corner).add_child(picture);
    }
    // "Source code (AGPL-3.0): <address>", the sentence About says too.
    let address = Phrase::SourceCode.fill(lang, &[source_address(state)]);
    let shown = commands
        .spawn((
            Text::new(address),
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
            Underline,
            UnderlineColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    // The address is the link where the gateway's passed the check at the
    // door; else it is text, as About says it.
    let source = if state.source_code.is_some() {
        let face = commands
            .spawn((
                Node {
                    padding: UiRect::axes(kit.m.px(6.0), px_fixed(2.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                    max_width: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER),
            ))
            .id();
        commands.entity(face).add_child(shown);
        let wrapper = controls::hit(
            commands,
            kit,
            face,
            (
                Press::Front(FrontPress::OpenSource),
                Stop::new(table, "source"),
            ),
        );
        commands
            .entity(wrapper)
            .entry::<Node>()
            .and_modify(|mut node| node.max_width = Val::Percent(100.0));
        wrapper
    } else {
        commands
            .entity(shown)
            .remove::<(Underline, UnderlineColor)>();
        shown
    };
    commands.entity(corner).add_child(source);
    corner
}

/// How wide the source corner may grow at the default step.
const SOURCE_MEASURE: f32 = 280.0;

/// The build this client is, on its own line under the text row, on every
/// face (owner, 09.10.2026: "below the bar below the form"): the whole
/// build on Wide and Vast, the version on the smaller classes.
pub(super) fn version_line(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let build = if matches!(kit.m.frame, Frame::Wide | Frame::Vast) {
        baylee_build::short()
    } else {
        baylee_build::VERSION
    };
    let row = plate(commands, kit);
    // "this client {0}", the version its own span (the tests and the eye
    // find the build whole).
    let said = Phrase::FrontThisClient.text(lang);
    let (before, after) = said.split_once("{0}").unwrap_or((said, ""));
    let words = commands
        .spawn((
            Text::new(before),
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    for part in [build, after] {
        if part.is_empty() {
            continue;
        }
        let span = commands
            .spawn((
                TextSpan::new(part),
                crate::hud::tf(kit.fonts, kit.m.small),
                TextColor(tokens::INK),
            ))
            .id();
        commands.entity(words).add_child(span);
    }
    commands.entity(row).add_child(words);
    row
}

/// The source address as a QR code on a framed plate; a press opens it.
fn qr(commands: &mut Commands, code: &super::super::source::Code) -> Entity {
    #[allow(clippy::cast_precision_loss)] // a code is at most 177 modules a side
    let side = px_fixed(code.side as f32 * super::super::source::MODULE_PX);
    let picture = commands
        .spawn((
            ImageNode::new(code.image.clone()),
            Node {
                width: side,
                height: side,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands
        .spawn((
            Node {
                padding: UiRect::all(px_fixed(3.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(5.0)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
            Button,
            Press::Front(FrontPress::OpenSource),
        ))
        .add_child(picture)
        .id()
}

/// The tagline on its mist plate.
pub(super) fn tagline(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    surfaces::mist(commands, kit, Phrase::WelcomeNote.text(state.lobby.lang()))
}

#[allow(clippy::too_many_lines)] // one sheet, read top to bottom
/// The About sheet (N4-8): every colophon sentence in full, the licence,
/// the source as a link and a code, and the third-party licences.
pub(crate) fn about(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    scrolled: &Scrolled,
) {
    if !state.about_open {
        return;
    }
    let lang = state.lobby.lang();
    let at = |id: &'static str| Stop::new(ABOUT.name, id);
    let phone = kit.m.frame == Frame::Phone;
    let surface = commands
        .spawn((
            Role::Opaque,
            Node {
                width: if phone {
                    Val::Percent(100.0)
                } else {
                    kit.m.px(720.0)
                },
                max_width: Val::Percent(100.0),
                max_height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: px_fixed(kit.m.gap),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(if phone {
                    0.0
                } else {
                    tokens::RADIUS_PANEL
                })),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
            Press::Shared(SharedPress::PickerNothing),
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(Phrase::AboutTitle.text(lang)),
            crate::hud::tf(kit.fonts, kit.m.h1),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let text = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_shrink: 1.0,
                min_height: px_fixed(0.0),
                row_gap: kit.m.px(10.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::About))),
            Scrollable(List::About),
            Role::Scroll,
            at("text"),
            Pickable::default(),
        ))
        .id();
    // Each paragraph as wide as the sheet's column, a definite width: at
    // another, a long one was given the height of fewer lines than it
    // broke into (the `fit` check, a phone at XL).
    let para = |commands: &mut Commands, words: &str, muted: bool| {
        let text = surfaces::prose(commands, kit, words, muted);
        commands.entity(text).insert(Node {
            width: Val::Percent(100.0),
            ..default()
        });
        text
    };
    let mut body = vec![
        para(commands, &format!("Baylee {}", baylee_build::short()), true),
        para(commands, FAN_CONTENT_NOTICE, false),
        para(commands, Phrase::ScryfallCredit.text(lang), false),
        para(commands, Phrase::AboutLicence.text(lang), false),
    ];
    let address = Phrase::SourceCode.fill(lang, &[source_address(state)]);
    let source = if state.source_code.is_some() {
        link(
            commands,
            kit,
            &address,
            Look::Underlined,
            (Press::Front(FrontPress::OpenSource), at("source")),
        )
    } else {
        para(commands, &address, false)
    };
    body.push(source);
    if let Some(code) = state.source_code.as_ref() {
        body.push(qr(commands, code));
    }
    body.push(surfaces::heading(
        commands,
        kit,
        Phrase::AboutThirdParty.text(lang),
    ));
    for phrase in [
        Phrase::AboutFonts,
        Phrase::AboutIcons,
        Phrase::AboutMana,
        Phrase::AboutCrates,
        Phrase::AboutPrecons,
    ] {
        body.push(para(commands, phrase.text(lang), false));
    }
    commands.entity(text).add_children(&body);
    let close = controls::button(
        commands,
        kit,
        Phrase::SheetClose.text(lang),
        Weight::Primary,
        Live::Yes,
        Some("Esc"),
        (Press::Front(FrontPress::About(false)), at("close")),
    );
    let foot = commands
        .spawn((
            Node {
                justify_content: JustifyContent::FlexEnd,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(foot).add_child(close);
    commands.entity(surface).add_children(&[title, text, foot]);
    let scrim = surfaces::sheet(commands, surface);
    commands.entity(root).add_child(scrim);
}
