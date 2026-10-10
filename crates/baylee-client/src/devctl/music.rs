//! Audition every suite/movement through the real persistent audio player.
use baylee_client_core::music::{Movement, Theme};
use bevy::prelude::*;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Preview {
    theme: Theme,
    movement: Movement,
}

pub(super) fn preview(body: &str, commands: &mut Commands) -> String {
    if serde_json::from_str::<serde_json::Value>(body)
        .is_ok_and(|value| value == serde_json::json!({"auto": true}))
    {
        commands.remove_resource::<crate::music::Audition>();
        return "{\"ok\":true,\"auto\":true}".to_string();
    }
    match serde_json::from_str::<Preview>(body) {
        Ok(preview) => {
            commands.insert_resource(crate::music::Audition(
                preview.movement.request(preview.theme),
            ));
            serde_json::json!({"ok":true,"theme":preview.theme,"movement":preview.movement,
                "rate":baylee_client_core::music::RATE})
            .to_string()
        }
        Err(error) => serde_json::json!({"error":error.to_string()}).to_string(),
    }
}
