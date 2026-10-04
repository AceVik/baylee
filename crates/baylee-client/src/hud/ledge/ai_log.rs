use super::*;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use serde_json::Value;

const LOG_PAD: f32 = 10.0;
const LOG_H: f32 = 420.0;

#[derive(Component)]
pub struct AiLogPanel;

#[derive(Resource)]
pub struct AiLogState {
    pub file_path: Option<PathBuf>,
    pub last_pos: u64,
}

impl Default for AiLogState {
    fn default() -> Self {
        Self {
            file_path: None,
            last_pos: 0,
        }
    }
}

pub(in crate::hud) fn spawn(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            AiLogPanel,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(hand::HAND_ZONE_H - STRIP_LIP + STRIP_H),
                right: px(EDGE + log::LOG_W + LOG_PAD),
                width: px(log::LOG_W),
                height: px(LOG_H),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::top(px(STRIP_R)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            ZIndex(Z_LOG),
            Visibility::Hidden,
        ))
        .id()
}

pub fn update_ai_log(
    mut state: ResMut<AiLogState>,
    duel: Res<Duel>,
    mut commands: Commands,
    mut panel_query: Query<(Entity, &mut Visibility), With<AiLogPanel>>,
    fonts: Res<UiFonts>,
) {
    let Some(statics) = duel.statics.as_ref() else {
        return;
    };
    
    if state.file_path.is_none() {
        for s in 1..=8 {
            let p = PathBuf::from(format!("target/seat-transcripts/{}-seat{}-mind.jsonl", statics.game_id, s));
            if p.exists() {
                state.file_path = Some(p);
                break;
            }
        }
    }

    let Some(path) = &state.file_path else { return; };
    let Ok(mut file) = File::open(path) else { return; };
    if file.seek(SeekFrom::Start(state.last_pos)).is_err() { return; }

    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut new_lines = Vec::new();
    
    while let Ok(bytes) = reader.read_line(&mut line) {
        if bytes == 0 { break; }
        if let Ok(json) = serde_json::from_str::<Value>(&line) {
            let event = json["event"].as_str().unwrap_or("");
            if event == "asked" {
                new_lines.push((Color::Srgba(bevy::color::palettes::css::ORANGE), format!("Prompt: {:?}", json["kind"])));
            } else if event == "answered" {
                let note = json["note"].as_str().unwrap_or("No thoughts provided.");
                let note_trimmed = if note.len() > 100 { format!("{}...", &note[..100]) } else { note.to_string() };
                new_lines.push((Color::Srgba(bevy::color::palettes::css::LIGHT_BLUE), format!("Answered: {}", note_trimmed)));
            } else if event == "refused" {
                new_lines.push((Color::Srgba(bevy::color::palettes::css::RED), format!("Refused: {}", json["reason"].as_str().unwrap_or("?"))));
            }
        }
        state.last_pos += bytes as u64;
        line.clear();
    }
    
    if new_lines.is_empty() { return; }

    if let Ok((panel, mut vis)) = panel_query.single_mut() {
        *vis = Visibility::Inherited;
        for (color, text) in new_lines {
            let row = commands.spawn((
                Text::new(text),
                super::tf(&fonts, 14.0),

                TextColor(color),
                Node {
                    margin: UiRect::all(px(4.0)),
                    ..default()
                },
            )).id();
            commands.entity(panel).add_child(row);
        }
    }
}
