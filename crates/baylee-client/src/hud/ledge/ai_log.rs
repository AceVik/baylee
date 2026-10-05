use bevy::prelude::*;
use super::*;
use baylee_protocol::v1;

const LOG_PAD: f32 = 10.0;
pub(in crate::hud) const LOG_H: f32 = 420.0;

#[derive(Component)]
pub struct AiLogPanel;

#[derive(Message, Clone)]
pub struct AiLogEvent {
    pub log: v1::AiLog,
}

pub(in crate::hud) fn spawn(commands: &mut Commands, fonts: &Res<UiFonts>) -> Entity {
    let id = commands
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
        .with_children(|parent| {
            parent.spawn((
                Text::new("--- AI LOG START ---".to_string()),
                super::tf(fonts, 14.0),
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::all(px(4.0)),
                    ..default()
                },
            ));
        })
        .id();
    id
}

pub fn update_ai_log(
    mut commands: Commands,
    mut panel_query: Query<(Entity, &mut Visibility), With<AiLogPanel>>,
    fonts: Res<UiFonts>,
    mut events: MessageReader<AiLogEvent>,
    duel: Res<Duel>,
) {
    let mut new_lines = Vec::new();
    for event in events.read() {
        let ai_log = &event.log;
        if ai_log.event == "asked" {
            new_lines.push((Color::Srgba(bevy::color::palettes::css::ORANGE), format!("Prompt: {}", ai_log.kind)));
        } else if ai_log.event == "answered" {
            if !ai_log.thinking.is_empty() {
                new_lines.push((Color::Srgba(bevy::color::palettes::css::GRAY), format!("Thinking:\n{}", ai_log.thinking)));
            }
            let note = &ai_log.note;
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(note) {
                let chose = parsed.get("chose").and_then(|v| v.as_str()).unwrap_or("?");
                let say = parsed.get("say").and_then(|v| v.as_str()).unwrap_or("");
                let tokens = parsed.get("tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let ms = parsed.get("ms").and_then(|v| v.as_u64()).unwrap_or(0);
                let time_sec = ms as f32 / 1000.0;
                
                new_lines.push((Color::Srgba(bevy::color::palettes::css::LIGHT_BLUE), format!("Action: {}", chose)));
                if !say.is_empty() {
                    new_lines.push((Color::WHITE, format!("\"{}\"", say)));
                }
                new_lines.push((Color::Srgba(bevy::color::palettes::css::DARK_GRAY), format!("({} tokens, {:.1}s)", tokens, time_sec)));
            } else {
                let note_trimmed = if note.len() > 100 { format!("{}...", &note[..100]) } else { note.to_string() };
                new_lines.push((Color::Srgba(bevy::color::palettes::css::LIGHT_BLUE), format!("Answered: {}", note_trimmed)));
            }
        } else if ai_log.event == "refused" {
            new_lines.push((Color::Srgba(bevy::color::palettes::css::RED), format!("Refused: {}", ai_log.reason)));
        }
    }
    
    let mut count = 0;
    for (panel, mut vis) in panel_query.iter_mut() {
        count += 1;
        let new_vis = if duel.ai_log_open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != new_vis {
            println!("AiLogPanel visibility changed to: {:?}", new_vis);
            *vis = new_vis;
        }

        for (color, text) in new_lines.iter() {
            let row = commands.spawn((
                Text::new(text.clone()),
                super::tf(&fonts, 14.0),
                TextColor(*color),
                Node {
                    margin: UiRect::all(px(4.0)),
                    ..default()
                },
            )).id();
            commands.entity(panel).add_child(row);
        }
    }
    
    if count == 0 && duel.ai_log_open {
        println!("WARNING: AiLog is open, but No AiLogPanel found!");
    }


}
