//! The ways out of a finished game, where the duel drew no end screen.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::ui_button::UiButton;

/// The bar the ways out fall back to when the duel drew no end screen.
///
/// Only ever on a node this module *made*, which is what keeps the teardown
/// honest: the buttons normally live inside the duel's sheet and are taken
/// down with it, and a marker that also sat on the duel's own row would have
/// this module despawning an entity the duel is about to despawn again.
#[derive(Component)]
pub(super) struct LeaveButton;

/// One way out of a finished game, wherever it ended up standing.
///
/// The "are these already placed" question, asked of the buttons rather than
/// of their holder for the same reason: the holder may be the duel's.
///
/// `pub(crate)` for the probe alone: `devctl`'s `exits` row reports whether
/// each on-screen `Press` also carries this, because
/// `systems::leave_keys` filters by it while `leave_clicks`
/// walks the clicked entity's ancestry, and a caller cannot otherwise tell a
/// way out the keyboard is blind to from one that is not there (#135).
#[derive(Component)]
pub(crate) struct DuelExit;

/// The ways out of a finished game, put in the row the duel's end screen left
/// for them.
///
/// An `Update` system and not an `OnEnter` one, and that is the whole of the
/// seam. `hud::spawn_finish` runs on the same edge, its `Commands` are applied
/// at the end of that schedule, and a system merely ordered *after* it would
/// query a row that does not exist yet — while an explicit sync point between
/// two plugins that do not know each other is exactly the coupling the marker
/// exists to avoid. So this runs every frame the game is over and stops the
/// moment its buttons are standing.
///
/// If there is no such row — the screen draws nothing without a roster, and an
/// embedder may have its own — the buttons fall back to a bar of their own
/// over the board, which is where they lived before the screen existed. A
/// player with no way out of a finished table is the one outcome worth a
/// fallback.
#[allow(clippy::too_many_lines)] // one row of ways, and the consent over it
pub(super) fn spawn_leave_button(
    mut commands: Commands,
    mut state: ResMut<LobbyState>,
    fonts: Option<Res<UiFonts>>,
    placed: Query<Entity, With<DuelExit>>,
    exits: Query<Entity, With<crate::hud::FinishExits>>,
    parents: Query<&ChildOf>,
) {
    let Some(fonts) = fonts else {
        return;
    };
    // The consent was flipped: the ways are drawn again with its new state.
    if state.replays.end_redraw {
        state.replays.end_redraw = false;
        for way in &placed {
            commands.entity(way).despawn();
        }
        return;
    }
    if !placed.is_empty() {
        return;
    }
    let lang = state.lobby.lang();
    // Play again first, because it is what most players want and the one that
    // needs the other three still at the table. Only for a game reached
    // through the gateway: an offline duel against the house has no table to
    // ask for another of, and the request would have no account to make it.
    //
    // `local` and not merely `Seated`, which is what this asked before and
    // which was wrong the whole time: an offline duel is seated too
    // (`systems::poll` reads `Screen::Seated(handover)` and branches on
    // `handover.local`), so playing the house put a *play again* over the
    // finished game that would have asked a gateway for another of a table
    // it has never heard of. It was floating over the board where nobody
    // looked; the end screen put it in the middle of the sheet.
    let networked = matches!(state.lobby.screen(), Screen::Seated(handover) if !handover.local && !handover.watching());
    // Watch replay: a game this client hosted, or a hosted one this player
    // sat in; never one only watched (desktop only).
    let replayable = super::replays::SHOWN
        && matches!(state.lobby.screen(), Screen::Seated(handover) if !handover.watching());
    let replay = matches!(state.lobby.screen(), Screen::Replay(_));
    let mut ways: Vec<(&str, Press)> = Vec::new();
    if replay {
        use baylee_client_core::replay::words::ReplayPhrase;
        ways.push((
            ReplayPhrase::WatchAgain.text(lang),
            Press::End(EndPress::WatchAgain),
        ));
        ways.push((
            ReplayPhrase::WatchFrom.text(lang),
            Press::End(EndPress::WatchFrom),
        ));
        ways.push((
            ReplayPhrase::BackToReplays.text(lang),
            Press::End(EndPress::Leave),
        ));
    } else {
        if networked {
            ways.push((
                Phrase::PlayAgain.text(lang),
                Press::End(EndPress::PlayAgain),
            ));
        }
        if replayable {
            ways.push((
                baylee_client_core::replay::words::ReplayPhrase::WatchReplay.text(lang),
                Press::End(EndPress::Replay),
            ));
        }
        ways.push((Phrase::BackToLobby.text(lang), Press::End(EndPress::Leave)));
    }

    // In the sheet these are the slip's own answers, so they obey the slip's
    // own rule: the first one is what the sheet is *for* and is the only one
    // in brass. That makes the lone "back to the lobby" of an offline duel a
    // lead answer, which is right — there is nothing left for it to be
    // quieter than.
    let holder = exits.iter().next().unwrap_or_else(|| {
        commands
            .spawn((
                LeaveButton,
                Node {
                    position_type: PositionType::Absolute,
                    top: px(64),
                    width: percent(100),
                    justify_content: JustifyContent::Center,
                    column_gap: px(12),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id()
    });
    for (i, (label, press)) in ways.into_iter().enumerate() {
        let way = crate::hud::answer_button(&mut commands, &fonts, label, i == 0);
        commands.entity(way).insert((press, DuelExit));
        commands.entity(holder).add_child(way);
    }
    // A hosted game this player sat in: whether the others may see their
    // hand and deck in its replays (§18.2), ticked unless they untick it
    // (owner, 13.10.2026), over the ways out.
    if let Screen::Seated(handover) = state.lobby.screen().clone()
        && super::replays::SHOWN
        && !handover.local
        && !handover.watching()
        && let Ok(parent) = parents.get(holder).map(ChildOf::parent)
    {
        let shared = state
            .replays
            .end_share
            .as_ref()
            .filter(|(game, _)| *game == handover.game_id)
            .is_none_or(|(_, shared)| *shared);
        let toggle = share_toggle(&mut commands, &fonts, lang, shared);
        commands
            .entity(toggle)
            .insert((Press::End(EndPress::Share), DuelExit));
        // Over the row of ways: the parent's children, the toggle placed
        // just before the row.
        commands.queue(move |world: &mut World| {
            let at = world
                .get::<Children>(parent)
                .and_then(|c| c.iter().position(|e| e == holder));
            if let Some(at) = at {
                world.entity_mut(parent).insert_children(at, &[toggle]);
            }
        });
    }
}

/// The end sheet's consent: a box and its words, on the parchment.
fn share_toggle(commands: &mut Commands, fonts: &UiFonts, lang: Lang, shared: bool) -> Entity {
    use baylee_client_core::replay::words::ReplayPhrase;
    let ink = crate::hud::palette::PARCHMENT_INK;
    commands
        .spawn((
            UiButton,
            Node {
                align_self: AlignSelf::Center,
                align_items: AlignItems::Center,
                column_gap: px(8),
                padding: UiRect::axes(px(10), px(6)),
                margin: UiRect::bottom(px(10)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            crate::ambience::Feel::tinting_to(
                Color::NONE,
                crate::hud::palette::PARCHMENT_EDGE.with_alpha(0.25),
            ),
        ))
        .with_children(|row| {
            // A drawn box: the icon face has a filled square only, which
            // reads as ticked either way.
            row.spawn((
                Node {
                    width: px(16),
                    height: px(16),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(3)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BorderColor::all(ink),
                BackgroundColor(if shared { ink } else { Color::NONE }),
                Pickable::IGNORE,
            ))
            .with_children(|tick| {
                if shared {
                    tick.spawn((
                        Text::new("\u{f00c}"),
                        crate::hud::icon_tf(fonts, 9.0),
                        TextColor(crate::hud::palette::PARCHMENT),
                        Pickable::IGNORE,
                    ));
                }
            });
            row.spawn((
                Text::new(ReplayPhrase::ShareHand.text(lang)),
                crate::hud::tf(fonts, 14.0),
                TextColor(ink),
                Pickable::IGNORE,
            ));
        })
        .id()
}

/// Removes it again on the way out.
pub(super) fn despawn_leave_button(
    mut commands: Commands,
    buttons: Query<Entity, With<LeaveButton>>,
) {
    for entity in &buttons {
        commands.entity(entity).despawn();
    }
}

/// One of the ways out of a finished game, which `leave_clicks` and
/// `leave_keys` take; `clicks` answers neither.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EndPress {
    /// Leave a finished game.
    Leave,
    /// Play that game again. Beside [`EndPress::Leave`], because those are the
    /// only two things left to do with a table that is over.
    PlayAgain,
    /// Watch the game just played again (a local game, or a hosted one this
    /// player sat in).
    Replay,
    /// A replay's end: from the start again.
    WatchAgain,
    /// A replay's end: from the next perspective it offers.
    WatchFrom,
    /// The consent over the ways: flip whether the others may see this
    /// player's hand and deck in the game's replays (§18.2). Not a way out:
    /// the table stays.
    Share,
}
