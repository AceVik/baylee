//! The builder's header (§7): the breadcrumb back to Decks and the deck's
//! name, the deck-wide actions, the **one** save state, the `⋯`, and the
//! shell's dot, bell and account on the right.
//!
//! The builder wears this in place of the shell's nav (§2.1: "replaces the
//! nav"); `1 2 3` do not work here and Esc never leaves (Back is `‹`).

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::lobby::{HeaderPress, LibraryPress};
use crate::shellkit::controls::Disabled;
use crate::shellkit::header as kit_header;
use baylee_client_core::deckbuilder::BuildField;

/// Marks the header's save state: the one node on the screen that says
/// whether the deck is saved (§2.5, WP4: "one save-state node").
#[derive(Component)]
pub(crate) struct SaveStateNode;

/// The account pill's handle: `GET /me`'s, a kept guest's, else an ellipsis
/// while `/me` is on its way; none offline.
pub(crate) fn handle(state: &LobbyState) -> Option<String> {
    if state.lobby.offline() {
        return None;
    }
    state
        .lobby
        .me()
        .map(|me| me.handle.clone())
        .filter(|h| !h.is_empty())
        .or_else(|| state.lobby.kept_guest().map(|g| g.handle.clone()))
        .or_else(|| Some("\u{2026}".to_string()))
}

/// Draws the header into `holder`.
#[allow(clippy::too_many_lines)] // one bar, read left to right
pub(super) fn draw(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let roomy = matches!(m.frame, Frame::Wide | Frame::Vast);
    let phone = m.frame == Frame::Phone;
    let bar = commands
        .spawn((
            Role::Header,
            Node {
                width: Val::Percent(100.0),
                min_height: px_fixed(m.header),
                padding: UiRect::axes(px_fixed(m.body), px_fixed(0.0)),
                column_gap: px_fixed(m.gap),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            crate::tour::TourAnchor(baylee_client_core::tour::Anchor::BuildHeader),
        ))
        .id();
    commands.entity(holder).add_child(bar);
    let mut kids: Vec<Entity> = Vec::new();

    // The brand, as the shell's header has it; a phone's header is too
    // short for the logo (§2.7).
    if !phone {
        let brand =
            crate::shellkit::header::brand_mark(commands, m.header, Phrase::AppName.text(lang));
        kids.push(brand);
        if roomy {
            kids.push(words(
                commands,
                kit,
                baylee_build::VERSION,
                m.small,
                tokens::MUTED,
            ));
        }
    }

    // ‹ Decks › name ✎
    let back = if roomy {
        controls::button(
            commands,
            kit,
            Phrase::BackToDecks.text(lang),
            Weight::Ghost,
            Live::Yes,
            None,
            (Press::Build(BuildPress::CloseBuilder), stop("back")),
        )
    } else {
        icon_button(
            commands,
            kit,
            mark::BACK,
            false,
            (Press::Build(BuildPress::CloseBuilder), stop("back")),
        )
    };
    kids.push(back);
    if roomy {
        kids.push(words(commands, kit, "\u{203a}", m.text, tokens::MUTED));
    }
    kids.push(title(commands, env));
    kids.push(spring(commands));

    if env.layout == Layout::PhoneSingle {
        let shown = usize::from(env.ui().pane != Pane::Pool);
        let switch = controls::segmented(
            commands,
            kit,
            &[
                Phrase::BuildPanePool.text(lang),
                Phrase::BuildPaneDeck.text(lang),
            ],
            shown,
            |i| {
                (
                    Press::Build(BuildPress::SetPane(if i == 0 {
                        Pane::Pool
                    } else {
                        Pane::Deck
                    })),
                    Stop::item(BUILDER, "panes", u8::try_from(i).unwrap_or(0)),
                    crate::shellkit::focus::Current(i == shown),
                )
            },
        );
        commands.entity(switch).insert(crate::tour::TourAnchor(
            baylee_client_core::tour::Anchor::BuildPaneSwitch,
        ));
        kids.push(switch);
    }

    if roomy {
        let busy = env.state.lobby.busy();
        for (text, press, id) in [
            (
                Phrase::ImportDeck,
                Press::Build(BuildPress::OpenImport),
                "import",
            ),
            (
                Phrase::ExportDeck,
                Press::Build(BuildPress::OpenExport),
                "export",
            ),
        ] {
            kids.push(controls::button(
                commands,
                kit,
                text.text(lang),
                Weight::Secondary,
                if busy {
                    Live::No(Phrase::SavingDeck.text(lang))
                } else {
                    Live::Yes
                },
                None,
                (press, stop(id)),
            ));
        }
        let history = env.state.lobby.token().is_some() && deck.editing().is_some();
        let why = if env.state.lobby.offline() {
            Phrase::HistoryNeedsGateway
        } else {
            Phrase::HistorySaveHint
        };
        kids.push(controls::button(
            commands,
            kit,
            Phrase::DeckHistory.text(lang),
            Weight::Secondary,
            if history && !busy {
                Live::Yes
            } else {
                Live::No(why.text(lang))
            },
            None,
            (Press::Library(LibraryPress::BrowseHistory), stop("history")),
        ));
    }

    kids.push(save_button(commands, env));
    kids.push(save_state(commands, env));
    let menu = icon_button(
        commands,
        kit,
        mark::MORE,
        env.ui().menu == Some(BuildMenu::Header),
        (
            Press::Build(BuildPress::ToggleHeaderMenu),
            stop("menu"),
            focus::MenuOpener(BuildMenu::Header),
        ),
    );
    kids.push(menu);

    // The shell's right end: the gateway's dot, the bell, the account.
    let reach = if env.state.lobby.offline() {
        tokens::MUTED
    } else if env.state.feed_down {
        tokens::DANGER
    } else {
        tokens::ACCENT
    };
    let dot = commands
        .spawn((
            Node {
                width: m.px(10.0),
                height: m.px(10.0),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(reach),
        ))
        .id();
    kids.push(controls::hit(
        commands,
        kit,
        dot,
        Press::Header(HeaderPress::Gateway),
    ));
    kids.push(kit_header::bell(
        commands,
        kit,
        env.state.bell.unread(),
        Press::Header(HeaderPress::Bell),
    ));
    if let Some(handle) = handle(env.state) {
        kids.push(kit_header::avatar(
            commands,
            kit,
            &handle,
            Press::Header(HeaderPress::Account),
        ));
    }
    commands.entity(bar).add_children(&kids);
    let _ = deck;
}

/// The deck's name, or its box while it is being renamed (F2, the pen).
fn title(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let deck = env.deck();
    let lang = env.lang();
    let renaming = env.ui().nav == Nav::Field && deck.focus() == BuildField::Name;
    if renaming {
        let field = crate::lobby::text_field(
            commands,
            kit.fonts,
            env.lobby_metrics(),
            "",
            &crate::lobby::FieldLook {
                buffer: deck.buffer(BuildField::Name),
                focused: true,
                mask: None,
                press: Press::Build(BuildPress::Rename),
                lead: None,
                hint: Some(Phrase::BuildUntitled.text(lang)),
                tail: None,
            },
        );
        commands.entity(field).insert((
            Node {
                width: m.px(if m.frame == Frame::Phone {
                    180.0
                } else {
                    280.0
                }),
                min_width: px_fixed(0.0),
                flex_shrink: 1.0,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            stop("title"),
        ));
        return field;
    }
    let name = deck.name().trim();
    let (text, ink) = if name.is_empty() {
        (Phrase::BuildUntitled.text(lang).to_string(), tokens::MUTED)
    } else {
        (name.to_string(), tokens::INK)
    };
    let face = commands
        .spawn((
            Node {
                min_width: px_fixed(0.0),
                max_width: m.px(360.0),
                flex_shrink: 1.0,
                column_gap: m.px(8.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let words = cell(commands, kit, &text, m.text * 1.1, ink, true);
    let pen = glyph(commands, kit, mark::PEN, m.small, tokens::MUTED);
    commands.entity(face).add_children(&[words, pen]);
    let hit = controls::hit(
        commands,
        kit,
        face,
        (Press::Build(BuildPress::Rename), stop("title")),
    );
    commands.entity(hit).insert(Node {
        min_width: px_fixed(0.0),
        min_height: px_fixed(kit.m.hit),
        flex_shrink: 1.0,
        align_items: AlignItems::Center,
        ..default()
    });
    hit
}

/// Save: the primary, with the unsaved dot and its key.
fn save_button(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let deck = env.deck();
    let live = deck.dirty() && deck.saveable() && !env.state.lobby.busy();
    let face = commands
        .spawn((
            Role::Button,
            Node {
                min_height: px_fixed(m.control),
                padding: UiRect::axes(m.px(if kit.german { 10.0 } else { 12.0 }), px_fixed(0.0)),
                column_gap: m.px(8.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(if live {
                tokens::PRIMARY
            } else {
                tokens::CONTROL.with_alpha(0.6)
            }),
            BorderColor::all(if live {
                tokens::PRIMARY_EDGE
            } else {
                tokens::BORDER
            }),
        ))
        .id();
    let label = commands
        .spawn((
            Text::new(Phrase::BuildSave.text(env.lang())),
            tf_bold(kit.fonts, m.text),
            TextColor(if live { tokens::INK } else { tokens::DISABLED }),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_child(label);
    if deck.dirty() {
        let dot = glyph(commands, kit, mark::DOT, m.small * 0.55, tokens::GOLD);
        commands.entity(face).add_child(dot);
    }
    if live
        && let Some(keys) = env.save_keys
        && let Some(cap) = controls::key_cap(commands, kit, keys)
    {
        commands.entity(face).add_child(cap);
    }
    if live {
        commands
            .entity(face)
            .insert(crate::ambience::Feel::new(tokens::PRIMARY));
        controls::hit(
            commands,
            kit,
            face,
            (
                Press::Build(BuildPress::SaveDeck),
                stop("save"),
                crate::tour::TourAnchor(baylee_client_core::tour::Anchor::BuildSave),
            ),
        )
    } else {
        controls::hit(
            commands,
            kit,
            face,
            (
                Press::Build(BuildPress::SaveDeck),
                stop("save"),
                Disabled,
                crate::tour::TourAnchor(baylee_client_core::tour::Anchor::BuildSave),
            ),
        )
    }
}

/// What the save state says: its mark, its ink, its words, and whether it
/// offers Retry.
fn sentence(env: &Env) -> (Option<char>, Color, String, bool) {
    let deck = env.deck();
    let lang = env.lang();
    match env.ui().save {
        SaveState::Saving => (
            None,
            tokens::MUTED,
            Phrase::BuildSaving.text(lang).into(),
            false,
        ),
        SaveState::Failed if deck.dirty() => (
            Some(mark::WARN),
            tokens::DANGER,
            Phrase::BuildSaveFailed.text(lang).into(),
            true,
        ),
        _ if deck.dirty() => {
            let n = deck.changes().max(1);
            let words = if n == 1 {
                Phrase::BuildUnsavedOne.text(lang).to_string()
            } else {
                Phrase::BuildUnsavedMany.fill(lang, &[&n.to_string()])
            };
            (Some(mark::DOT), tokens::GOLD, words, false)
        }
        SaveState::Saved { minutes, .. } => (
            Some(mark::CHECK),
            tokens::INK,
            if minutes == 0 {
                Phrase::BuildSavedNow.text(lang).to_string()
            } else {
                Phrase::BuildSavedAgo.fill(lang, &[&minutes.to_string()])
            },
            false,
        ),
        _ if deck.editing().is_some() => (
            Some(mark::CHECK),
            tokens::MUTED,
            Phrase::BuildSaved.text(lang).into(),
            false,
        ),
        _ => (
            None,
            tokens::MUTED,
            Phrase::BuildNotSaved.text(lang).into(),
            false,
        ),
    }
}

/// The one save state (§7: "Unsaved · n changes / Saved ✓ · time /
/// Couldn't save · Retry"); on a narrow header its mark alone.
fn save_state(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let (sign_mark, ink, said, retry) = sentence(env);
    let node = commands
        .spawn((
            SaveStateNode,
            // Its mark never gives way; its sentence may (the cell clips).
            Node {
                align_items: AlignItems::Center,
                column_gap: m.px(6.0),
                min_width: px_fixed(m.small * 1.2),
                flex_shrink: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(sign_mark) = sign_mark {
        let size = if sign_mark == mark::DOT {
            m.small * 0.55
        } else {
            m.small
        };
        let sign = glyph(commands, kit, sign_mark, size, ink);
        commands.entity(node).add_child(sign);
    }
    let room = matches!(m.frame, Frame::Wide | Frame::Vast | Frame::Narrow);
    if room {
        let text = cell(commands, kit, &said, m.small, ink, false);
        commands.entity(node).add_child(text);
    }
    if retry {
        let again = link(
            commands,
            kit,
            Phrase::ShellRetry.text(env.lang()),
            (Press::Build(BuildPress::SaveDeck), stop("retry")),
        );
        commands.entity(node).add_child(again);
    }
    node
}
