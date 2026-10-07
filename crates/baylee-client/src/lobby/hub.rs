//! The signed-in hub: Play and Decks, side by side or stacked.

use super::clicks::sign_out;
use super::press::Cx;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// The signed-in screen: decks and tables, side by side or stacked.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)] // the screen's inputs, passed down in order
pub(super) fn table(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
    kit: crate::shellkit::controls::Kit,
    prefs: &baylee_client_core::prefs::Preferences,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let phone = metrics.frame == Frame::Compact;

    if phone {
        commands
            .entity(root)
            .entry::<Node>()
            .and_modify(|mut n| n.overflow = Overflow::scroll_y());
        commands.entity(root).insert((
            Scrollable(List::Table),
            ScrollPosition(Vec2::new(0.0, scrolled_to.get(List::Table))),
        ));
    }

    // Use the full viewport for discovery and multiplayer configuration.
    let frame = commands
        .spawn((
            Node {
                width: percent(100),
                height: if phone { Val::Auto } else { percent(100) },
                min_height: if phone { percent(100) } else { px(0) },
                flex_shrink: 0.0,
                align_self: AlignSelf::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(frame);
    let root = frame;

    // ---- the shell's header, its strips and popovers (WP0b-3)
    super::header::draw(commands, root, state, kit, metrics);
    // What the lobby last said, under the header: refusals land here.
    if !lobby.status().is_empty() {
        let line = commands
            .spawn((
                Node {
                    width: percent(100),
                    justify_content: JustifyContent::FlexEnd,
                    flex_shrink: 0.0,
                    padding: UiRect::axes(px(metrics.pad), px(2)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        // On a mist plate: no text stands on the painting bare (§2.2).
        let plate = commands
            .spawn((
                Node {
                    padding: UiRect::axes(px(8), px(2)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(crate::shellkit::tokens::MIST),
                Pickable::IGNORE,
            ))
            .id();
        let status = commands
            .spawn((
                Text::new(lobby.status()),
                tf(fonts, metrics.small),
                TextColor(status_ink(lobby.tone())),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(plate).add_child(status);
        commands.entity(line).add_child(plate);
        commands.entity(root).add_child(line);
    }

    if let Some(handover) = lobby.awaiting().filter(|_| !state.room_away)
        && let Some(index) = lobby
            .games()
            .iter()
            .position(|g| g.id == handover.game_id && g.state == "waiting")
    {
        super::room::draw(
            commands,
            root,
            state,
            fonts,
            metrics,
            kit,
            scrolled_to,
            index,
        );
        return;
    }

    if let Some(handover) = lobby.awaiting() {
        let banner = commands
            .spawn((
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.5)),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                Pickable::IGNORE,
            ))
            .id();
        // Written as a phrase and not a `format!`, which is what it was:
        // a hand-typed English sentence renders a German screen half in
        // English, and it does it silently — the compile error that a
        // missing translation is arrives only for text that goes through
        // `Phrase`.
        let words = if lobby.offline() {
            Phrase::TableOpenHouseWaiting.text(lang).to_string()
        } else {
            Phrase::TableOpenWaiting.fill(lang, &[&short_id(&handover.game_id)])
        };
        let line = commands
            .spawn((
                Text::new(words),
                tf(fonts, metrics.small),
                TextColor(palette::ACTIVE),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(banner).add_child(line);
        commands.entity(root).add_child(banner);
    }

    // A guest is told what a guest is, for as long as it plays as one (#269):
    // the account and its decks go when its session lapses, about thirty
    // days after its last visit (the expiry slides on every request, not
    // only on a game), or at once when it signs out.
    if lobby.guest() {
        let banner = commands
            .spawn((
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.5)),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                Pickable::IGNORE,
            ))
            .id();
        let line = commands
            .spawn((
                Text::new(Phrase::GuestNotice.text(lang)),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(banner).add_child(line);
        commands.entity(root).add_child(banner);
    }

    // The two screens under the header (WP2, WP3): the header's nav is
    // the way between them.
    match state.hub {
        Hub::Play => super::play::draw(commands, root, state, metrics, kit, scrolled_to),
        Hub::Decks => super::decks::draw(commands, root, state, prefs, metrics, kit, scrolled_to),
    }
}

/// A house AI's difficulty, in the player's own language.
///
/// The name itself stays the gateway's word — it is what `RoomPress::SeatAi`
/// sends and what `SeatSpec` stores; only the label is translated. The
/// lookup is [`baylee_client_core::i18n::ai_name`] rather than a `match`
/// here, because the *table* needs the same answer this list gives and did
/// not have it: a chair arranged here as "Solide" sat down called
/// `steady 1`.
///
/// An unknown spelling keeps this list's own long-standing answer — the
/// middle difficulty — because a row in a lobby always draws something and
/// the caller above already defaults a missing value to `"steady"`.
pub(super) fn ai_name(lang: Lang, name: &str) -> &'static str {
    baylee_client_core::i18n::ai_name(lang, name).unwrap_or_else(|| Phrase::AiSteady.text(lang))
}

/// The head of an opaque game id — enough to tell two tables apart, and short
/// enough to fit on a phone.
fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

/// A control of the signed-in hub that is neither Play's nor Decks' own:
/// which of the two shows, signing out, re-reading, the pager.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum HubPress {
    /// Show one of the hub's two screens.
    Tab(Hub),
    /// Forget the account.
    SignOut,
    /// Re-read decks and tables (F5, the shell's Refresh).
    Refresh,
    /// Step one page through the table list. `true` is forwards.
    Page(bool),
}

impl HubPress {
    /// What a click on this control does.
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx {
            state,
            prefs,
            scrolled,
            mailbox,
            settings,
        } = cx;
        match self {
            HubPress::Tab(hub) => {
                if state.hub != hub {
                    state.hub = hub;
                    scrolled.set(List::Table, 0.0);
                }
            }
            // A guest is asked first: signed out, it is gone (#269).
            HubPress::SignOut if state.lobby.guest() => {
                state.confirmation = Some(confirm::Destructive::SignOutGuest);
            }
            HubPress::SignOut => {
                sign_out(state, prefs, scrolled, mailbox, settings);
            }
            HubPress::Refresh => {
                let request = state.lobby.refresh();
                dispatch(state, mailbox, request);
            }
            HubPress::Page(forwards) => {
                let request = state.lobby.page(forwards);
                dispatch(state, mailbox, request);
            }
        }
    }
}
