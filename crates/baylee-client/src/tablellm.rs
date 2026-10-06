//! The in-game panel of a debug build's language-model chairs
//! (`docs/llm-seat.md` §"Changing a chair during the game"): for each chair
//! a bridge of this client plays, the house or another profile, model or
//! effort, from the bridge's next decision. A release build has no panel
//! ([`TableLlmPlugin`] is added only where `LIVE_CHANGES`), and its bridge
//! refuses an order besides.
//!
//! The panel is rebuilt only when what it shows changed
//! (`TableSeats::revision`), never per frame; it stands only while a
//! chair is planned, so a table with no language model of this client's
//! shows nothing.

use crate::hud::{ButtonWeight, UiFonts, answer_sized, palette, tf};
use crate::tableseats::{LlmPress, Phase};
use baylee_client_core::i18n::Phrase;
use baylee_client_core::llmseat::models::Resolved;
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui::px;

/// Adds the panel to a debug build's duel.
pub struct TableLlmPlugin;

impl Plugin for TableLlmPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (draw, clicks).run_if(in_state(crate::DuelPhase::Playing)),
        )
        .add_systems(OnExit(crate::DuelPhase::Playing), despawn);
    }
}

/// The panel's root.
#[derive(Component)]
struct LlmPanel;

/// What a press on the panel asks for the chair.
#[derive(Component, Clone, Copy)]
struct GamePress(u32, LlmPress);

/// Rebuilds the panel when what it shows changed.
fn draw(
    mut commands: Commands,
    state: Option<Res<crate::lobby::LobbyState>>,
    fonts: Option<Res<UiFonts>>,
    panels: Query<Entity, With<LlmPanel>>,
    mut drawn: Local<Option<u64>>,
) {
    let (Some(state), Some(fonts)) = (state, fonts) else {
        return;
    };
    let revision = state.llm.revision;
    if *drawn == Some(revision) && (panels.is_empty() == state.llm.chairs().is_empty()) {
        return;
    }
    *drawn = Some(revision);
    for panel in &panels {
        commands.entity(panel).despawn();
    }
    let chairs = state.llm.chairs();
    if chairs.is_empty() {
        return;
    }
    let lang = state.lobby.lang();
    let root = commands
        .spawn((
            LlmPanel,
            Node {
                position_type: PositionType::Absolute,
                top: px(72),
                left: px(12),
                max_width: px(460),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(10)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            GlobalZIndex(crate::hud::Z_LOG),
        ))
        .id();
    let title = line(
        &mut commands,
        &fonts,
        Phrase::GameLlmTitle.text(lang),
        palette::DIALOG_INK,
    );
    commands.entity(root).add_child(title);
    for chair in chairs {
        chair_rows(&mut commands, &fonts, root, &state.llm, lang, chair);
    }
}

/// One chair's rows: what it plays, and the house, its profiles, models
/// and efforts to change to.
fn chair_rows(
    commands: &mut Commands,
    fonts: &UiFonts,
    root: Entity,
    llm: &crate::tableseats::TableSeats,
    lang: baylee_client_core::i18n::Lang,
    chair: u32,
) {
    let Some(planned) = llm.planned(chair) else {
        return;
    };
    let current = llm.current(chair);
    let caption = current
        .as_ref()
        .map_or_else(|| planned.model.clone(), Resolved::caption);
    let effort = planned
        .effort
        .clone()
        .unwrap_or_else(|| Phrase::RoomLlmEffortOwn.text(lang).to_string());
    let head = if llm.house_now(chair) {
        format!("{} · {}", chair + 1, Phrase::GameLlmHouse.text(lang))
    } else {
        format!(
            "{} · {}",
            chair + 1,
            Phrase::RoomLlmPlays.fill(lang, &[&caption, &effort])
        )
    };
    let head = line(commands, fonts, &head, palette::DIALOG_INK);
    commands.entity(root).add_child(head);
    let mut items = vec![(
        Phrase::GameLlmHouse.text(lang).to_string(),
        LlmPress::House,
        llm.house_now(chair),
    )];
    items.extend(llm.profiles().iter().enumerate().map(|(at, (name, _))| {
        (
            (*name).to_string(),
            LlmPress::Profile(at),
            !llm.house_now(chair) && *name == planned.profile,
        )
    }));
    chips(commands, fonts, root, chair, &items);
    let models: Vec<(String, LlmPress, bool)> = llm
        .models(chair)
        .iter()
        .enumerate()
        .map(|(at, m)| (m.caption(), LlmPress::Model(at), m.id == planned.model))
        .collect();
    chips(commands, fonts, root, chair, &models);
    if let Some(current) = &current {
        let mut efforts = vec![(
            Phrase::RoomLlmEffortOwn.text(lang).to_string(),
            LlmPress::Effort(None),
            planned.effort.is_none(),
        )];
        efforts.extend(current.efforts.iter().enumerate().map(|(at, e)| {
            (
                (*e).to_string(),
                LlmPress::Effort(Some(at)),
                planned.effort.as_deref() == Some(*e),
            )
        }));
        chips(commands, fonts, root, chair, &efforts);
    }
    if let Some(said) = llm.said(chair) {
        let said = line(commands, fonts, &said, palette::DIALOG_SOFT);
        commands.entity(root).add_child(said);
    }
}

/// A line of text.
fn line(commands: &mut Commands, fonts: &UiFonts, words: &str, ink: Color) -> Entity {
    commands
        .spawn((
            Text::new(words),
            tf(fonts, 13.0),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id()
}

/// A wrapping row of buttons for `chair`, the lit one weighted.
fn chips(
    commands: &mut Commands,
    fonts: &UiFonts,
    parent: Entity,
    chair: u32,
    items: &[(String, LlmPress, bool)],
) {
    let row = commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(4),
                row_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (words, press, on) in items {
        let weight = if *on {
            ButtonWeight::Candle
        } else {
            ButtonWeight::Secondary
        };
        let id = answer_sized(commands, fonts, words, weight, None, 26.0, 12.0);
        commands.entity(id).insert(GamePress(chair, *press));
        commands.entity(row).add_child(id);
    }
    commands.entity(parent).add_child(row);
}

/// Does what a press on the panel asks: an order to the chair's bridge.
fn clicks(
    mut pointer: MessageReader<Pointer<Click>>,
    presses: Query<&GamePress>,
    parents: Query<&ChildOf>,
    state: Option<ResMut<crate::lobby::LobbyState>>,
) {
    let Some(mut state) = state else {
        pointer.clear();
        return;
    };
    for click in pointer.read() {
        if let Some(GamePress(chair, press)) =
            crate::input::find_in_lineage(click.entity, &presses, &parents).copied()
        {
            state.llm.press(chair, press, Phase::Playing, "steady");
        }
    }
}

/// Takes the panel away with the duel.
fn despawn(mut commands: Commands, panels: Query<Entity, With<LlmPanel>>) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
}
