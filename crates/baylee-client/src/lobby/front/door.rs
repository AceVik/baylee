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

/// Whether the full colophon (and the QR) stands under the card.
pub(super) fn full_colophon(kit: Kit) -> bool {
    matches!(kit.m.frame, Frame::Wide | Frame::Vast)
}

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
    let face = commands
        .spawn((
            Role::Button,
            Node {
                padding: UiRect::axes(kit.m.px(8.0), px_fixed(0.0)),
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
    commands
        .entity(face)
        .insert((Pickable::default(), crate::ambience::Feel::new(Color::NONE)));
    wrapper
}

/// A separator dot between links.
fn sep(commands: &mut Commands, kit: Kit) -> Entity {
    controls::label(commands, kit, "\u{b7}", kit.m.small, tokens::INK)
}

/// The text row on its mist plate: the languages (one radio group), the
/// music, Settings, Play offline, About (A6: what the gear used to hide).
pub(super) fn text_row(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    music_on: bool,
) -> Entity {
    let lang = state.lobby.lang();
    let row = plate(commands, kit);
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
        let s = sep(commands, kit);
        let item = link(
            commands,
            kit,
            text,
            Look::Plain,
            (press, Stop::new(table, id)),
        );
        items.extend([s, item]);
    }
    commands.entity(row).add_children(&items);
    row
}

/// The one-line colophon (§3): the build, the notice shortened in the
/// policy's own words (the full text one press away, on About), Scryfall's
/// credit, the source link.
pub(super) fn one_line(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let table = keys::table_of(state).name;
    let row = plate(commands, kit);
    let build = controls::label(
        commands,
        kit,
        baylee_build::short(),
        kit.m.small,
        tokens::INK,
    );
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
    // The address itself stays in sight, not only behind the link: the
    // AGPL's offer is read where every player passes (`docs/legal.md` §6).
    // A link only for an address that passed the check at the door.
    let address = Phrase::SourceCode.fill(lang, &[source_address(state)]);
    let source = if state.source_code.is_some() {
        link(
            commands,
            kit,
            &address,
            Look::Underlined,
            (
                Press::Front(FrontPress::OpenSource),
                Stop::new(table, "source"),
            ),
        )
    } else {
        let said = controls::label(commands, kit, &address, kit.m.small, tokens::INK);
        commands
            .entity(said)
            .insert(TextLayout::new(Justify::Center, LineBreak::WordOrCharacter));
        said
    };
    let parts = [build, notice, credit, source];
    for (i, part) in parts.into_iter().enumerate() {
        if i > 0 {
            let s = sep(commands, kit);
            commands.entity(row).add_child(s);
        }
        commands.entity(row).add_child(part);
    }
    row
}

/// The full colophon (Wide, Vast): the build, the notice word for word,
/// Scryfall's credit and the source link on one mist plate, the QR at the
/// right when there is a plain address to encode (#299).
pub(super) fn full(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let table = keys::table_of(state).name;
    let holder = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
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
                max_width: Val::Percent(70.0),
                padding: UiRect::axes(kit.m.px(12.0), kit.m.px(6.0)),
                row_gap: kit.m.px(2.0),
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
    let build = line(commands, baylee_build::short());
    let notice = line(commands, FAN_CONTENT_NOTICE);
    let credit = line(commands, Phrase::ScryfallCredit.text(lang));
    let address = Phrase::SourceCode.fill(lang, &[source_address(state)]);
    let source = if state.source_code.is_some() {
        link(
            commands,
            kit,
            &address,
            Look::Underlined,
            (
                Press::Front(FrontPress::OpenSource),
                Stop::new(table, "source"),
            ),
        )
    } else {
        let said = line(commands, &address);
        commands
            .entity(said)
            .insert(TextLayout::new(Justify::Center, LineBreak::WordOrCharacter));
        said
    };
    commands
        .entity(words)
        .add_children(&[build, notice, credit, source]);
    commands.entity(holder).add_child(words);
    if let Some(code) = state.source_code.as_ref() {
        let picture = qr(commands, code);
        commands.entity(picture).insert(Node {
            position_type: PositionType::Absolute,
            right: kit.m.px(12.0),
            bottom: kit.m.px(10.0),
            padding: UiRect::all(px_fixed(3.0)),
            border: UiRect::all(px_fixed(1.0)),
            border_radius: BorderRadius::all(px_fixed(5.0)),
            ..default()
        });
        commands.entity(holder).add_child(picture);
    }
    holder
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
            at("text"),
            Pickable::default(),
        ))
        .id();
    let para = |commands: &mut Commands, words: &str, muted: bool| {
        surfaces::prose(commands, kit, words, muted)
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
