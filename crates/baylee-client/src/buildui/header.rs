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
use save::save_button;

/// Marker component kept for test queries (`save_states`).
#[derive(Component)]
#[allow(dead_code)]
pub(crate) struct SaveStateNode;

/// Slot inside the builder header bar that holds the deck title / rename box
/// so entering or leaving rename mode updates in place without rebuilding the
/// entire header bar.
#[derive(Component)]
pub(crate) struct HeaderTitleSlot;

/// Spawns a circular header icon button styled identically to the Settings
/// gear button (`32×32` `RADIUS_PILL` with hover lift and a single `HoverHint`).
pub(super) fn tool_icon_button(
    commands: &mut Commands,
    kit: Kit,
    icon: char,
    enabled: bool,
    hint: String,
    trailing: &[Entity],
    action: impl Bundle,
) -> Entity {
    let ink = if enabled {
        tokens::INK
    } else {
        tokens::DISABLED
    };
    let face = commands
        .spawn((
            Role::Button,
            Node {
                min_width: kit.m.px(32.0),
                min_height: kit.m.px(32.0),
                padding: UiRect::axes(kit.m.px(6.0), kit.m.px(4.0)),
                column_gap: kit.m.px(3.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            UiTransform::default(),
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(icon.to_string()),
            icon_tf(kit.fonts, kit.m.text),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(face)
        .add_child(glyph)
        .add_children(trailing);
    let wrapper = if enabled {
        commands
            .entity(face)
            .insert(crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER));
        controls::hit(commands, kit, face, action)
    } else {
        controls::hit(commands, kit, face, (action, controls::Disabled))
    };
    commands
        .entity(wrapper)
        .insert(crate::lobby::HoverHint(hint));
    wrapper
}

/// Replaces only the [`HeaderTitleSlot`]'s children in place when the deck
/// name or rename mode changes.
pub(super) fn patch_title(commands: &mut Commands, env: &Env) {
    let content = title(commands, env);
    commands.queue(move |world: &mut World| {
        let mut slots = world.query_filtered::<Entity, With<HeaderTitleSlot>>();
        if let Some(slot) = slots.iter(world).next() {
            world.entity_mut(slot).despawn_related::<Children>();
            world.entity_mut(slot).add_child(content);
        } else if let Ok(entity) = world.get_entity_mut(content) {
            entity.despawn();
        }
    });
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
    let bar = kit_header::frame(commands, kit);
    commands.entity(bar).insert(crate::tour::TourAnchor(
        baylee_client_core::tour::Anchor::BuildHeader,
    ));
    commands.entity(holder).add_child(bar);
    let mut kids: Vec<Entity> = Vec::new();

    // The brand, as the shell's header has it; a phone's header is too
    // short for the logo (§2.7). Clicking opens the memorial dialog.
    if !phone {
        let brand =
            crate::shellkit::header::brand_mark(commands, m.header, Phrase::AppName.text(lang));
        let memorial_hint = Phrase::MemorialInvitation.text(lang).to_string();
        commands.entity(brand).insert((
            Role::Hit,
            bevy::picking::hover::PickingInteraction::default(),
            Pickable::default(),
            Press::Memorial(crate::lobby::memorial::MemorialPress::Open),
            crate::lobby::HoverHint(memorial_hint),
        ));
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
    let title_slot = commands
        .spawn((
            HeaderTitleSlot,
            Node {
                align_items: AlignItems::Center,
                min_width: px_fixed(0.0),
                flex_shrink: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title_node = title(commands, env);
    commands.entity(title_slot).add_child(title_node);
    kids.push(title_slot);
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

    if !phone {
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
            let hint = if busy {
                format!("{} — {}", text.text(lang), Phrase::SavingDeck.text(lang))
            } else {
                text.text(lang).to_string()
            };
            let btn = tool_icon_button(commands, kit, icon, !busy, hint, &[], (press, stop(id)));
            kids.push(btn);
        }
        let history = env.state.lobby.token().is_some() && deck.editing().is_some();
        let why = if env.state.lobby.offline() {
            Phrase::HistoryNeedsGateway
        } else {
            Phrase::HistorySaveHint
        };
        let hist_enabled = history && !busy;
        let hist_hint = if hist_enabled {
            Phrase::DeckHistory.text(lang).to_string()
        } else {
            format!(
                "{} — {}",
                Phrase::DeckHistory.text(lang),
                if busy {
                    Phrase::SavingDeck.text(lang)
                } else {
                    why.text(lang)
                }
            )
        };
        let hist_btn = tool_icon_button(
            commands,
            kit,
            crate::hud::glyph::CLOCK,
            hist_enabled,
            hist_hint,
            &[],
            (Press::Library(LibraryPress::BrowseHistory), stop("history")),
        );
        kids.push(hist_btn);

        let del_hint = if busy {
            format!(
                "{} / {} — {}",
                Phrase::EmptyTheDeck.text(lang),
                Phrase::Delete.text(lang),
                Phrase::SavingDeck.text(lang)
            )
        } else {
            format!(
                "{} / {}",
                Phrase::EmptyTheDeck.text(lang),
                Phrase::Delete.text(lang)
            )
        };
        let del_btn = tool_icon_button(
            commands,
            kit,
            '\u{f1f8}',
            !busy,
            del_hint,
            &[],
            (
                Press::Build(BuildPress::ToggleHeaderMenu),
                stop("delete"),
                super::focus::MenuOpener(BuildMenu::Header),
            ),
        );
        kids.push(del_btn);
    }

    kids.push(save_button(commands, env));

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
            bell: (
                Press::Header(HeaderPress::Bell),
                crate::lobby::HoverHint(Phrase::HeaderNotifications.text(lang).to_string()),
            ),
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
        let row = commands
            .spawn((
                Node {
                    align_items: AlignItems::Center,
                    column_gap: m.px(6.0),
                    min_width: px_fixed(0.0),
                    flex_shrink: 1.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
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
                    240.0
                } else {
                    480.0
                }),
                min_width: px_fixed(0.0),
                flex_shrink: 1.0,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            stop("title"),
        ));
        let save_name = tool_icon_button(
            commands,
            kit,
            '\u{f00c}',
            true,
            Phrase::BuildSave.text(lang).to_string(),
            &[],
            Press::Build(BuildPress::FinishRename),
        );
        commands.entity(row).add_children(&[field, save_name]);
        return row;
    }
    let name = deck.name().trim();
    let (text, ink) = if name.is_empty() {
        (Phrase::BuildUntitled.text(lang).to_string(), tokens::MUTED)
    } else {
        (name.to_string(), tokens::INK)
    };
    let row = commands
        .spawn((
            Node {
                min_width: px_fixed(0.0),
                max_width: m.px(480.0),
                flex_shrink: 1.0,
                column_gap: m.px(4.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let words = cell(commands, kit, &text, m.text * 1.1, ink, true);
    commands
        .entity(words)
        .insert(Press::Build(BuildPress::Rename));
    let edit_btn = tool_icon_button(
        commands,
        kit,
        mark::PEN,
        true,
        Phrase::BuildRename.text(lang).to_string(),
        &[],
        (Press::Build(BuildPress::Rename), stop("title")),
    );
    commands.entity(row).add_children(&[words, edit_btn]);
    row
}
