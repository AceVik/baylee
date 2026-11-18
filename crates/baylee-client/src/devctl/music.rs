//! Audition every suite/movement through the real persistent audio player,
//! and the recorded soundtrack's scenes and stingers.
use baylee_client_core::music::{Movement, SampleSet, Scene, Theme};
use bevy::prelude::*;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Preview {
    theme: Theme,
    movement: Movement,
    #[serde(default)]
    samples: SampleSet,
    /// Play the recorded soundtrack's bed for this movement instead.
    #[serde(default)]
    recorded: bool,
    /// The Tower's scene (a lobby movement only): the Tower's bed.
    #[serde(default)]
    tower: bool,
    /// Fire a stinger over the table's bed: `"call"` or `"epic"`.
    #[serde(default)]
    stinger: Option<StingerKind>,
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum StingerKind {
    Call,
    Epic,
}

/// Each audition's stinger moves the counter on, so the audio side plays it.
static STINGERS: AtomicU8 = AtomicU8::new(0);

pub(super) fn preview(body: &str, commands: &mut Commands) -> String {
    if serde_json::from_str::<serde_json::Value>(body)
        .is_ok_and(|value| value == serde_json::json!({"auto": true}))
    {
        commands.remove_resource::<crate::music::Audition>();
        return "{\"ok\":true,\"auto\":true}".to_string();
    }
    match serde_json::from_str::<Preview>(body) {
        Ok(preview) => {
            let mut request = preview.movement.request(preview.theme);
            request.samples = preview.samples;
            request.recorded = preview.recorded && baylee_client_core::music::RECORDED;
            if preview.tower && request.scene == Scene::Lobby {
                request.scene = Scene::Tower;
            }
            request.stingers = STINGERS.load(Ordering::Relaxed);
            if let Some(kind) = preview.stinger {
                request.stingers = STINGERS.fetch_add(1, Ordering::Relaxed).wrapping_add(1) & 7;
                request.stinger_epic = matches!(kind, StingerKind::Epic);
            }
            commands.insert_resource(crate::music::Audition(request));
            serde_json::json!({"ok":true,"theme":preview.theme,"movement":preview.movement,
                "samples":preview.samples,"recorded":request.recorded,"scene":format!("{:?}", request.scene),
                "stinger":preview.stinger,
                "recorded_source_rate":44100,"synthesis_rate":48000,"rate":baylee_client_core::music::RATE})
            .to_string()
        }
        Err(error) => serde_json::json!({"error":error.to_string()}).to_string(),
    }
}
