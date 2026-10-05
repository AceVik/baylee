//! The AI log: what a teammate's AI mind said beside its answers.
//!
//! The engine forwards a seat's reasoning (`v1::AiLog`) only to a teammate
//! the table shows that seat's hand to (`docs/protocol.md` §"An AI seat's
//! reasoning"), so whatever arrives here this seat was entitled to. The lines
//! are worked out in `baylee_client_core::aisaid`; this draws them, in a
//! panel beside the game log that opens from its own door in the tray. The
//! door stands only once something has been said ([`Duel::ai_log_heard`]),
//! so a table without an AI teammate shows no door to an empty panel.

use super::*;
use baylee_client_core::aisaid::{self, Kind};
use baylee_protocol::v1;

const LOG_PAD: f32 = 10.0;
const LOG_H: f32 = 420.0;
const LINE_PT: f32 = 13.0;

/// The panel.
#[derive(Component)]
pub struct AiLogPanel;

/// One reasoning the host passed on, for [`update_ai_log`] to draw.
#[derive(Message, Clone)]
pub struct AiLogEvent {
    /// What arrived.
    pub log: v1::AiLog,
}

/// Spawns the panel, hidden; [`update_ai_log`] shows it while it is open.
pub(in crate::hud) fn spawn(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            AiLogPanel,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(hand::HAND_ZONE_H - hand::LEDGE_H + LEDGE_PAD_Y),
                right: px(EDGE + menu::BURGER + 6.0 + log::LOG_W + LOG_PAD),
                width: px(log::LOG_W),
                height: px(LOG_H),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::top(px(STRIP_R)),
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(px(10.0)),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            ZIndex(Z_LOG),
            Visibility::Hidden,
            bevy::ui::ScrollPosition::default(),
        ))
        .id()
}

/// The ink a line is drawn in.
fn ink(kind: Kind) -> Color {
    match kind {
        Kind::Head => palette::DIALOG_INK,
        Kind::Thinking | Kind::Cost => palette::DIALOG_SOFT,
        Kind::Chose | Kind::Say => palette::LEDGE_SOFT,
    }
}

/// Shows or hides the panel, and appends each reasoning that arrived.
pub fn update_ai_log(
    mut commands: Commands,
    mut panels: Query<(Entity, &mut Visibility), With<AiLogPanel>>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    mut events: MessageReader<AiLogEvent>,
    duel: Res<Duel>,
) {
    let lang = Lang::of(&settings.lang);
    let lines: Vec<aisaid::Line> = events
        .read()
        .flat_map(|event| {
            let seat = u8::try_from(event.log.seat).unwrap_or(u8::MAX);
            let who = baylee_client_core::i18n::seat_name(
                lang,
                duel.statics.as_ref(),
                PlayerId::new(seat),
            );
            aisaid::lines(lang, &who, &event.log.note, &event.log.thinking)
        })
        .collect();
    let shown = if duel.ai_log_open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (panel, mut seen) in &mut panels {
        if *seen != shown {
            *seen = shown;
        }
        for line in &lines {
            let row = commands
                .spawn((
                    Text::new(line.text.clone()),
                    tf(&fonts, LINE_PT),
                    TextColor(ink(line.kind)),
                    Node {
                        margin: UiRect::all(px(4.0)),
                        ..default()
                    },
                ))
                .id();
            commands.entity(panel).add_child(row);
        }
    }
}
