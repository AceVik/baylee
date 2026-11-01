//! The builder's header (§7): the breadcrumb back to Decks and the deck's
//! name, the deck-wide actions, the **one** save state, the `⋯`, and the
//! shell's dot, quick settings, social tools, bell and account on the right.
//!
//! The builder wears this in place of the shell's nav (§2.1: "replaces the
//! nav"); `1 2 3` do not work here and Esc never leaves (Back is `‹`).

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::lobby::{HeaderPress, LibraryPress, handle, tools};
use crate::shellkit::header as kit_header;
use baylee_client_core::deckbuilder::BuildField;

mod save;
use save::{save_button, save_state};

/// Marks the header's save state: the one node on the screen that says
/// whether the deck is saved (§2.5, WP4: "one save-state node").
#[derive(Component)]
pub(crate) struct SaveStateNode;

/// Draws the header into `holder`.
#[allow(clippy::too_many_lines)] // one bar, read left to right
pub(super) fn draw(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let roomy = matches!(m.frame, Frame::Wide | Frame::Vast);
    let phone = m.frame == Frame::Phone;
    let bar = kit_header::frame(commands, kit);
    commands.entity(bar).insert(crate::tour::TourAnchor(
        baylee_client_core::tour::Anchor::BuildHeader,
    ));
    commands.entity(holder).add_child(bar);
    let mut kids: Vec<Entity> = Vec::new();

    // The brand, as the shell's header has it; a phone's header is too
    // short for the logo (§2.7).
    if !phone {
        let brand =
            crate::shellkit::header::brand_mark(commands, m.header, Phrase::AppName.text(lang));
        kids.push(brand);
    }

    // ‹ Decks › name ✎
    let back = controls::button_compact(
        commands,
        kit,
        Some(mark::BACK),
        if roomy {
            Phrase::ShellDecks.text(lang)
        } else {
            ""
        },
        Weight::Ghost,
        Live::Yes,
        (Press::Build(BuildPress::CloseBuilder), stop("back")),
    );
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
        for (text, icon, press, id) in [
            (
                Phrase::ImportDeck,
                '\u{f56f}',
                Press::Build(BuildPress::OpenImport),
                "import",
            ),
            (
                Phrase::ExportDeck,
                '\u{f56e}',
                Press::Build(BuildPress::OpenExport),
                "export",
            ),
        ] {
            kids.push(controls::button_compact(
                commands,
                kit,
                Some(icon),
                text.text(lang),
                Weight::Secondary,
                if busy {
                    Live::No(Phrase::SavingDeck.text(lang))
                } else {
                    Live::Yes
                },
                (press, stop(id)),
            ));
        }
        let history = env.state.lobby.token().is_some() && deck.editing().is_some();
        let why = if env.state.lobby.offline() {
            Phrase::HistoryNeedsGateway
        } else {
            Phrase::HistorySaveHint
        };
        kids.push(controls::button_compact(
            commands,
            kit,
            Some(crate::hud::glyph::CLOCK),
            Phrase::DeckHistory.text(lang),
            Weight::Secondary,
            if history && !busy {
                Live::Yes
            } else {
                Live::No(why.text(lang))
            },
            (Press::Library(LibraryPress::BrowseHistory), stop("history")),
        ));
    }

    kids.push(save_button(commands, env));
    kids.push(save_state(commands, env));
    let menu = controls::button_compact(
        commands,
        kit,
        Some(mark::MORE),
        "",
        if env.ui().menu == Some(BuildMenu::Header) {
            Weight::Secondary
        } else {
            Weight::Ghost
        },
        Live::Yes,
        (
            Press::Build(BuildPress::ToggleHeaderMenu),
            stop("menu"),
            focus::MenuOpener(BuildMenu::Header),
        ),
    );
    kids.push(menu);

    commands.entity(bar).add_children(&kids);
    let reach = if env.state.lobby.offline() {
        kit_header::Reach::Offline
    } else if env.state.feed_down {
        kit_header::Reach::Down
    } else {
        kit_header::Reach::Up
    };
    let handle = handle(env.state);
    let tools = tools(commands, env.state, kit);
    kit_header::status(
        commands,
        kit,
        bar,
        &kit_header::HeaderLook {
            brand: Phrase::AppName.text(lang),
            build: "",
            nav: &[],
            active: None,
            reach,
            gateway: "",
            counts: "",
            unread: env.state.bell.unread(),
            handle: handle.as_deref(),
            return_pill: None,
            tools: &tools,
        },
        kit_header::StatusActions {
            gateway: Press::Header(HeaderPress::Gateway),
            bell: Press::Header(HeaderPress::Bell),
            account: Press::Header(HeaderPress::Account),
            back: Press::Header(HeaderPress::Return),
        },
        true,
    );
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
