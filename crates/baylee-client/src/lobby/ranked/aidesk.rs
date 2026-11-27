//! The desk's AI-player pieces (`docs/proposals/ai-players.md` §"Joining a
//! search"): whom a search accepts, the fill-in countdown, and the badge of
//! an AI opponent a match found or a result names.
//!
//! **Nothing jumps.** The countdown's room stands from the search's first
//! second ([`fill_in_slot`]); [`animate`] writes into it in place, fading it
//! in for the countdown's last fifteen seconds and out before it would say
//! zero, from the clock rather than from a component, so a redrawn desk
//! carries on where the last one stood. The choice's line keeps two lines'
//! room whichever choice it explains, and the control and its line stand
//! below the search's Leave, where a change of either moves nothing above.
use super::{Action, Kit, LobbyState, ORDER, Phrase, Press, Weight, controls, parts, tokens};
use crate::hud::glyph;
use crate::shellkit::{controls::Live, focus::Stop};
use baylee_client_core::aiplayers::{AiBadge, AiOpponents, FillIn, Opponents};
use bevy::prelude::*;

/// The desk's choice of whom a search accepts: a caption, the three
/// choices as one segmented control (none filled before the person chose),
/// and the line that says what the chosen one means.
pub(super) fn opponents(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    chosen: Option<Opponents>,
) {
    let lang = kit.lang();
    let caption = parts::caption(commands, kit, Phrase::RankedOpponents.text(lang));
    commands.entity(parent).add_child(caption);
    let items: Vec<&str> = Opponents::ALL
        .iter()
        .map(|choice| choice.title().text(lang))
        .collect();
    // Nothing is filled until the person chose: no index is `usize::MAX`.
    let selected = chosen.map_or(usize::MAX, Opponents::index);
    let control = controls::segmented(commands, kit, &items, selected, |i| {
        (
            Press::Ranked(Action::Opponents(Opponents::ALL[i])),
            Stop::item(ORDER.name, "opponents", u8::try_from(i).unwrap_or(u8::MAX)),
        )
    });
    commands.entity(parent).add_child(control);
    // Every meaning stands in one grid cell, the chosen one seen and the
    // others hidden but laid out, so the room is as tall as the longest in
    // this language and text step and a change of choice moves nothing.
    let room = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(1, 1.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let chosen_meaning = chosen.map_or(Phrase::RankedOpponentsUnset, Opponents::meaning);
    for meaning in [Phrase::RankedOpponentsUnset]
        .into_iter()
        .chain(Opponents::ALL.map(Opponents::meaning))
    {
        let line = parts::line(
            commands,
            kit,
            meaning.text(lang),
            kit.m.small,
            if meaning == Opponents::Any.meaning() {
                tokens::INK
            } else {
                tokens::MUTED
            },
        );
        commands.entity(line).insert((
            Node {
                grid_row: GridPlacement::start(1),
                grid_column: GridPlacement::start(1),
                ..default()
            },
            if meaning == chosen_meaning {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        ));
        commands.entity(room).add_child(line);
    }
    commands.entity(parent).add_child(room);
}

/// The fill-in countdown's room on a waiting search: the robot, "AI
/// opponent in 0:12", the kind the search accepts, and a one-click People
/// only. Hidden but standing until the countdown begins.
#[derive(Component)]
pub(super) struct FillInSlot;

/// The countdown's words.
#[derive(Component)]
pub(super) struct FillInWords;

/// The kind the search accepts, after the words.
#[derive(Component)]
pub(super) struct FillInKind;

/// A text's own ink in the countdown, which the fade scales.
#[derive(Component, Clone, Copy)]
pub(super) struct FadeInk(Color);

pub(super) fn fill_in_slot(commands: &mut Commands, parent: Entity, kit: Kit) {
    let lang = kit.lang();
    let slot = commands
        .spawn((
            FillInSlot,
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                column_gap: kit.m.px(6.0),
                min_height: Val::Px(kit.m.hit),
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(parent).add_child(slot);
    let mark = commands
        .spawn((
            Text::new(glyph::HOUSE.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small),
            TextColor(tokens::ACCENT),
            FadeInk(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            FillInWords,
            Text::new(""),
            counting_bold(kit),
            TextColor(tokens::ACCENT),
            FadeInk(tokens::ACCENT),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    let kind = commands
        .spawn((
            FillInKind,
            Text::new(""),
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            FadeInk(tokens::MUTED),
            TextLayout::no_wrap(),
            Node {
                flex_shrink: 1.0,
                min_width: Val::Px(0.0),
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let grow = parts::grow(commands);
    let people = controls::button_compact(
        commands,
        kit,
        Some(glyph::PERSON),
        Opponents::People.title().text(lang),
        Weight::Ghost,
        Live::Yes,
        Press::Ranked(Action::PeopleOnly),
    );
    commands
        .entity(slot)
        .add_children(&[mark, words, kind, grow, people]);
}

/// A caption and an AI opponent's badge, on a match found ("Opponent") or
/// a result ("Against").
pub(super) fn opponent_line(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    caption: Phrase,
    badge: &AiBadge,
) {
    let lang = kit.lang();
    let row = parts::row(commands, kit, false);
    let words = parts::line(
        commands,
        kit,
        caption.text(lang),
        kit.m.small,
        tokens::MUTED,
    );
    let pill = crate::aibadge::pill(commands, kit, badge, kit.m.small, true);
    commands.entity(row).add_children(&[words, pill]);
    commands.entity(parent).add_child(row);
}

/// Figures that keep their width as they count: lining and tabular.
fn tabular() -> bevy::text::FontFeatures {
    bevy::text::FontFeatures::builder()
        .enable(bevy::text::FontFeatureTag::LINING_FIGURES)
        .enable(bevy::text::FontFeatureTag::TABULAR_FIGURES)
        .build()
}

/// The search's clock face: small, with tabular figures, so "0:59" and
/// "1:00" are as wide as each other.
pub(super) fn counting(kit: Kit) -> TextFont {
    let mut font = crate::hud::tf(kit.fonts, kit.m.small);
    font.font_features = tabular();
    font
}

/// The countdown's face: bold, tabular.
fn counting_bold(kit: Kit) -> TextFont {
    let mut font = crate::hud::tf_bold(kit.fonts, kit.m.small);
    font.font_features = tabular();
    font
}

/// The words after the countdown: the kind the search accepts, when this
/// client sent it (a search found at launch does not say).
fn kind_words(state: &LobbyState) -> String {
    let lang = state.lobby.lang();
    match state.ranked_net.ticket_choice() {
        Some(AiOpponents::Local) => format!("· {}", Opponents::Local.title().text(lang)),
        Some(AiOpponents::Any) => format!("· {}", Opponents::Any.title().text(lang)),
        Some(AiOpponents::None) | None => String::new(),
    }
}

/// Writes the countdown into its room: its opacity from Central's clock,
/// its words once a second, the kind after them. Every write is compared
/// first, so a desk at rest writes nothing.
#[allow(clippy::type_complexity)] // two text roles, told apart by marker
pub(super) fn animate(
    mut commands: Commands,
    state: Res<LobbyState>,
    prefs: Res<crate::prefs::Prefs>,
    mut slots: Query<(Entity, &mut Visibility), With<FillInSlot>>,
    children: Query<&Children>,
    mut inks: Query<(&mut TextColor, Option<&FadeInk>)>,
    mut texts: Query<(&mut Text, Has<FillInWords>), Or<(With<FillInWords>, With<FillInKind>)>>,
) {
    if slots.is_empty() {
        return;
    }
    let net = &state.ranked_net;
    let at = net.fill_in_at();
    let now_ms = net.central_now_ms();
    let fill = FillIn::at(now_ms.div_euclid(1000), at);
    let opacity = if prefs.all().reduce_motion {
        if matches!(fill, FillIn::Counting(_)) {
            1.0
        } else {
            0.0
        }
    } else {
        FillIn::opacity(now_ms, at)
    };
    // The words change once a second and stay as they were outside the
    // countdown, so the fade out shows its last second, never "0:00".
    let said = fill.words(state.lobby.lang());
    let kind = kind_words(&state);
    for (mut text, is_words) in &mut texts {
        let next = if is_words { said.as_ref() } else { Some(&kind) };
        if let Some(next) = next
            && text.0 != *next
        {
            text.0.clone_from(next);
        }
    }
    for (slot, mut shown) in &mut slots {
        // Every text in the room is faded from its own ink; the Leave-
        // for-people button's texts are told theirs on first sight, and the
        // room stays hidden that frame so nothing shows at full strength.
        let mut ready = true;
        for text in children.iter_descendants(slot) {
            let Ok((mut ink, base)) = inks.get_mut(text) else {
                continue;
            };
            let Some(base) = base.copied() else {
                commands.entity(text).insert(FadeInk(ink.0));
                ready = false;
                continue;
            };
            let next = base.0.with_alpha(base.0.alpha() * opacity);
            if ink.0 != next {
                ink.0 = next;
            }
        }
        let visible = if ready && opacity > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *shown != visible {
            *shown = visible;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn waiting(at: Option<i64>) -> baylee_account::ranked::QueueState {
        baylee_account::ranked::QueueState::Waiting {
            key: [1; 16],
            format: baylee_account::ranked::Format::Standard,
            queued_at: 0,
            ai_fill_in_at: at,
        }
    }

    /// The countdown's room is hidden before the countdown, shows during it
    /// with its words, and the same frames twice write nothing.
    #[test]
    fn the_countdown_shows_in_its_room_and_rests_between_seconds() {
        let mut app = App::new();
        let mut state = LobbyState::new();
        let now = state.ranked_net.central_now();
        state.ranked_net.queue = waiting(Some(now + 9));
        app.insert_resource(state)
            .insert_resource(crate::prefs::Prefs::default());
        let words = app
            .world_mut()
            .spawn((
                FillInWords,
                Text::new(""),
                TextColor(tokens::ACCENT),
                FadeInk(tokens::ACCENT),
            ))
            .id();
        let slot = app
            .world_mut()
            .spawn((FillInSlot, Node::default(), Visibility::Hidden))
            .add_child(words)
            .id();
        app.add_systems(Update, animate);
        app.update();
        let said = app.world().get::<Text>(words).unwrap().0.clone();
        assert!(said.contains("0:0"), "{said}");
        assert_eq!(
            *app.world().get::<Visibility>(slot).unwrap(),
            Visibility::Inherited
        );
        // People only: no fill-in time, and the room hides.
        app.world_mut()
            .resource_mut::<LobbyState>()
            .ranked_net
            .queue = waiting(None);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(slot).unwrap(),
            Visibility::Hidden
        );
        let ink = app.world().get::<TextColor>(words).unwrap().0;
        assert!(
            ink.alpha() < f32::EPSILON,
            "faded out, not left at full ink"
        );
    }

    #[test]
    fn far_from_the_fill_in_the_room_stands_hidden() {
        let mut app = App::new();
        let mut state = LobbyState::new();
        let now = state.ranked_net.central_now();
        state.ranked_net.queue = waiting(Some(now + 40));
        app.insert_resource(state)
            .insert_resource(crate::prefs::Prefs::default());
        let slot = app
            .world_mut()
            .spawn((FillInSlot, Node::default(), Visibility::Hidden))
            .id();
        app.add_systems(Update, animate);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(slot).unwrap(),
            Visibility::Hidden
        );
    }
}
