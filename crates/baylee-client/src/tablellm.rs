//! The in-game panel of a debug build's language-model chairs
//! (`docs/llm-seat.md` §"Changing a chair during the game"): for each chair
//! a bridge of this client plays, the house or another profile, model or
//! effort, from the bridge's next decision. A release build has no panel
//! ([`TableLlmPlugin`] is added only where `LIVE_CHANGES`), and its bridge
//! refuses an order besides.
//!
//! The panel is rebuilt only when what it shows changed
//! (`TableSeats::revision`, or folding it), never per frame; it stands only
//! while a chair is planned, so a table with no language model of this
//! client's shows nothing.
//!
//! It opens folded: one button in the square beside the report button
//! (`hud::BESIDE_CORNER`), which lies on no seat's place, and only a press
//! on it lays the panel over the table, below `hud::TOP_CLEAR` at the right.

use crate::hud::{
    BESIDE_CORNER, ButtonWeight, CORNER_BUTTON, EDGE, TOP_CLEAR, UiFonts, answer_sized, btn_radius,
    palette, tf,
};
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
        app.init_resource::<Unfolded>()
            .add_systems(
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

/// The button the panel folds to.
#[derive(Component)]
struct Fold;

/// Whether the panel is laid out over the table; folded to its button
/// otherwise, as every game begins.
#[derive(Resource, Default)]
struct Unfolded(bool);

/// Rebuilds the panel when what it shows changed.
fn draw(
    mut commands: Commands,
    state: Option<ResMut<crate::lobby::LobbyState>>,
    fonts: Option<Res<UiFonts>>,
    panels: Query<Entity, With<LlmPanel>>,
    unfolded: Res<Unfolded>,
    mut drawn: Local<Option<(u64, bool)>>,
) {
    use bevy::prelude::DetectChangesMut as _;
    let (Some(mut state), Some(fonts)) = (state, fonts) else {
        return;
    };
    // The room's systems stand still during the game: a bridge's new line
    // is heard here, and only a new one rebuilds the panel.
    state.bypass_change_detection().llm.listen();
    let shown = (state.llm.revision, unfolded.0);
    if *drawn == Some(shown) && (panels.is_empty() == state.llm.chairs().is_empty()) {
        return;
    }
    *drawn = Some(shown);
    for panel in &panels {
        commands.entity(panel).despawn();
    }
    let chairs = state.llm.chairs();
    if chairs.is_empty() {
        return;
    }
    fold(&mut commands, &fonts, unfolded.0);
    if !unfolded.0 {
        return;
    }
    let lang = state.lobby.lang();
    let root = commands
        .spawn((
            LlmPanel,
            Node {
                position_type: PositionType::Absolute,
                top: px(TOP_CLEAR),
                right: px(EDGE),
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

/// The button beside the report button that folds and unfolds the panel,
/// lit while it is unfolded.
fn fold(commands: &mut Commands, fonts: &UiFonts, unfolded: bool) {
    let (ground, ink) = if unfolded {
        (palette::DIALOG_LINE, palette::DIALOG_INK)
    } else {
        (palette::DIALOG, palette::DIALOG_SOFT)
    };
    commands.spawn((
        LlmPanel,
        Fold,
        Node {
            position_type: PositionType::Absolute,
            right: px(BESIDE_CORNER),
            top: px(EDGE),
            width: px(CORNER_BUTTON),
            height: px(CORNER_BUTTON),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(1)),
            border_radius: btn_radius(),
            ..default()
        },
        BackgroundColor(ground),
        BorderColor::all(palette::DIALOG_LINE),
        Button,
        GlobalZIndex(crate::hud::Z_LOG),
        children![(
            // The kind of mind, as the chair's own name begins.
            Text::new("LLM"),
            tf(fonts, 10.0),
            TextColor(ink),
            Pickable::IGNORE,
        )],
    ));
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
        llm.lit(chair, LlmPress::House),
    )];
    items.extend(llm.profiles().iter().enumerate().map(|(at, (name, _))| {
        (
            (*name).to_string(),
            LlmPress::Profile(at),
            llm.lit(chair, LlmPress::Profile(at)),
        )
    }));
    chips(commands, fonts, root, chair, &items);
    let models: Vec<(String, LlmPress, bool)> = llm
        .models(chair)
        .iter()
        .enumerate()
        .map(|(at, m)| {
            (
                m.caption(),
                LlmPress::Model(at),
                llm.lit(chair, LlmPress::Model(at)),
            )
        })
        .collect();
    chips(commands, fonts, root, chair, &models);
    if let Some(current) = &current {
        let mut efforts = vec![(
            Phrase::RoomLlmEffortOwn.text(lang).to_string(),
            LlmPress::Effort(None),
            llm.lit(chair, LlmPress::Effort(None)),
        )];
        efforts.extend(current.efforts.iter().enumerate().map(|(at, e)| {
            (
                (*e).to_string(),
                LlmPress::Effort(Some(at)),
                llm.lit(chair, LlmPress::Effort(Some(at))),
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
            // Its width bounded, so a long line wraps inside the panel and
            // the panel grows to hold it.
            Node {
                max_width: px(440),
                ..default()
            },
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
    folds: Query<(), With<Fold>>,
    parents: Query<&ChildOf>,
    state: Option<ResMut<crate::lobby::LobbyState>>,
    mut unfolded: ResMut<Unfolded>,
) {
    let Some(mut state) = state else {
        pointer.clear();
        return;
    };
    for click in pointer.read() {
        if folds.contains(click.entity)
            || parents
                .get(click.entity)
                .is_ok_and(|parent| folds.contains(parent.parent()))
        {
            unfolded.0 = !unfolded.0;
            continue;
        }
        if let Some(GamePress(chair, press)) =
            crate::input::find_in_lineage(click.entity, &presses, &parents).copied()
        {
            state.llm.press(chair, press, Phase::Playing, "steady");
        }
    }
}

/// Takes the panel away with the duel, folded for the next.
fn despawn(
    mut commands: Commands,
    panels: Query<Entity, With<LlmPanel>>,
    mut unfolded: ResMut<Unfolded>,
) {
    for panel in &panels {
        commands.entity(panel).despawn();
    }
    // The next game begins folded.
    unfolded.0 = false;
}
