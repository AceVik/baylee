//! Procedural rosewood and chime sound effects for Tower Run (`Echoes`) and its
//! table equipment drawer (`docs/legal.md` §5: zero shipped audio files).
//!
//! Built on [`super::render`] so every Tower cue shares the same Just-Intonation
//! D-anchored rosewood bar, contact filter and stereo Hadamard room as the duel,
//! staying harmonically consonant with the Tavern music and never clipping.

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy::prelude::*;

use super::{Recipe, Strike, Where, blow, mallet, note, render, source, wav};

/// Every distinct interaction or confirmed progression moment in Tower Run.
// Browser Echoes is coming soon; desktop `lobby::echoes::native` emits the lobby cues.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TowerSound {
    /// Opening Echoes of the Tower.
    Open,
    /// Switching between Overview, Collection, Ranking, or Bag filters/pages.
    Tab,
    /// Selecting a relic, starter preview, or reward card.
    Select,
    /// Enrolling in a new expedition with a chosen Commander.
    DraftStart,
    /// Authoritative shop purchase confirmed.
    Buy,
    /// Authoritative relic sale confirmed.
    Sell,
    /// Authoritative merchant shelf reroll confirmed.
    Reroll,
    /// Freezing the merchant shelf.
    Freeze,
    /// Unfreezing the merchant shelf.
    Thaw,
    /// Equipping or swapping a relic into an active loadout slot.
    Equip,
    /// Storing a relic back into the inventory bag.
    Store,
    /// Automatic 3-of-a-kind fusion or Mythic potion ascension.
    Fusion,
    /// Post-duel reward / loot chest unveiled.
    RewardReveal,
    /// Claiming chosen reward cards into the expedition pool.
    RewardClaim,
    /// Entering or resuming a Tower duel.
    StageEnter,
    /// Toggling the in-duel Run Equipment drawer.
    DrawerOpen,
    /// Choosing an active potion or relic ability in a duel.
    ItemUse,
    /// Authority conflict or unavailable action feedback.
    Refused,
}

impl TowerSound {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 17] = [
        Self::Open,
        Self::Tab,
        Self::Select,
        Self::DraftStart,
        Self::Buy,
        Self::Sell,
        Self::Reroll,
        Self::Freeze,
        Self::Thaw,
        Self::Equip,
        Self::Store,
        Self::Fusion,
        Self::RewardReveal,
        Self::RewardClaim,
        Self::StageEnter,
        Self::DrawerOpen,
        Self::ItemUse,
        // Note: Refused is tested separately below too!
    ];
}

const E4: f32 = 330.37;
const B4: f32 = 495.55;
const E5: f32 = 660.74;
const FS5: f32 = 734.16;
const A5: f32 = 880.98;

const OPEN_STRIKES: [Strike; 3] = [
    blow(0.000, note::D3, 0.75, mallet::SOFT, 0.18, 0.72),
    blow(0.085, note::A3, 0.80, mallet::SOFT, -0.22, 0.86),
    blow(0.175, note::D4, 0.90, mallet::SOFT, 0.12, 1.00),
];
const TAB_STRIKES: [Strike; 1] = [blow(0.000, note::A4, 0.42, mallet::YARN, 0.28, 1.00)];
const SELECT_STRIKES: [Strike; 1] = [blow(0.000, note::D5, 0.48, mallet::SOFT, -0.20, 1.00)];
const DRAFT_STRIKES: [Strike; 4] = [
    blow(0.000, note::D4, 0.80, mallet::SOFT, 0.15, 0.75),
    blow(0.075, note::FS4, 0.82, mallet::SOFT, -0.18, 0.82),
    blow(0.150, note::A4, 0.88, mallet::SOFT, 0.24, 0.90),
    blow(0.235, note::D5, 1.00, mallet::HARD, -0.10, 1.00),
];
const BUY_STRIKES: [Strike; 3] = [
    blow(0.000, note::A4, 0.60, mallet::SOFT, 0.20, 0.82),
    blow(0.058, note::D5, 0.72, mallet::HARD, -0.16, 0.95),
    blow(0.122, A5, 0.85, mallet::HARD, 0.10, 0.78),
];
const SELL_STRIKES: [Strike; 3] = [
    blow(0.000, note::D5, 0.58, mallet::SOFT, -0.18, 0.88),
    blow(0.068, note::A4, 0.65, mallet::SOFT, 0.22, 0.92),
    blow(0.142, note::FS4, 0.78, mallet::HARD, -0.12, 0.85),
];
const REROLL_STRIKES: [Strike; 5] = [
    blow(0.000, note::D5, 0.40, mallet::SOFT, 0.32, 0.78),
    blow(0.052, B4, 0.42, mallet::SOFT, -0.26, 0.82),
    blow(0.098, note::A4, 0.45, mallet::SOFT, 0.18, 0.86),
    blow(0.140, note::FS4, 0.50, mallet::SOFT, -0.14, 0.92),
    blow(0.178, note::D4, 0.68, mallet::SOFT, 0.10, 1.00),
];
const FREEZE_STRIKES: [Strike; 3] = [
    blow(0.000, note::A4, 0.75, mallet::HARD, 0.12, 0.76),
    blow(0.042, E5, 0.85, mallet::HARD, -0.16, 0.92),
    blow(0.095, A5, 0.95, mallet::HARD, 0.08, 0.84),
];
const THAW_STRIKES: [Strike; 3] = [
    blow(0.000, E5, 0.55, mallet::YARN, -0.18, 0.84),
    blow(0.065, note::A4, 0.65, mallet::SOFT, 0.20, 0.90),
    blow(0.135, note::D4, 0.78, mallet::YARN, -0.12, 1.00),
];
const EQUIP_STRIKES: [Strike; 2] = [
    blow(0.000, note::A3, 0.62, mallet::SOFT, 0.16, 0.88),
    blow(0.055, E4, 0.78, mallet::HARD, -0.14, 1.00),
];
const EQUIP_THUDS: [(f32, f32); 1] = [(0.000, 0.22)];
const STORE_STRIKES: [Strike; 2] = [
    blow(0.000, E4, 0.50, mallet::YARN, -0.20, 0.86),
    blow(0.060, note::A3, 0.62, mallet::YARN, 0.18, 1.00),
];
const FUSION_STRIKES: [Strike; 5] = [
    blow(0.000, note::D4, 0.85, mallet::SOFT, 0.22, 0.68),
    blow(0.078, note::FS4, 0.88, mallet::SOFT, -0.18, 0.76),
    blow(0.152, note::A4, 0.92, mallet::HARD, 0.14, 0.85),
    blow(0.228, note::D5, 0.98, mallet::HARD, -0.10, 0.94),
    blow(0.315, FS5, 1.00, mallet::HARD, 0.06, 1.00),
];
const REWARD_REVEAL_STRIKES: [Strike; 4] = [
    blow(0.000, note::D4, 0.88, mallet::SOFT, 0.14, 0.80),
    blow(0.018, note::A4, 0.88, mallet::SOFT, -0.20, 0.78),
    blow(0.135, note::FS4, 0.95, mallet::SOFT, 0.16, 0.88),
    blow(0.155, note::D5, 1.00, mallet::HARD, -0.12, 1.00),
];
const REWARD_CLAIM_STRIKES: [Strike; 3] = [
    blow(0.000, note::D5, 0.55, mallet::SOFT, 0.24, 0.82),
    blow(0.068, E5, 0.62, mallet::SOFT, -0.18, 0.90),
    blow(0.142, A5, 0.85, mallet::HARD, 0.10, 1.00),
];
const STAGE_ENTER_STRIKES: [Strike; 3] = [
    blow(0.000, note::D3, 0.95, mallet::SOFT, 0.10, 0.90),
    blow(0.095, note::A3, 0.98, mallet::SOFT, -0.16, 0.94),
    blow(0.205, note::D4, 1.00, mallet::HARD, 0.08, 1.00),
];
const DRAWER_STRIKES: [Strike; 2] = [
    blow(0.000, note::D4, 0.45, mallet::YARN, 0.22, 0.82),
    blow(0.048, note::A4, 0.52, mallet::SOFT, -0.18, 1.00),
];
const ITEM_USE_STRIKES: [Strike; 2] = [
    blow(0.000, note::FS4, 0.68, mallet::SOFT, 0.16, 0.86),
    blow(0.062, note::CS5, 0.82, mallet::HARD, -0.14, 1.00),
];
const REFUSED_THUDS: [(f32, f32); 2] = [(0.000, 1.00), (0.058, 0.72)];

const fn strike_recipe(strikes: &'static [Strike], len: f32, peak: f32, room: Where) -> Recipe {
    Recipe {
        strikes,
        thuds: &[],
        len,
        peak,
        room,
    }
}

const fn recipe(kind: TowerSound) -> Recipe {
    match kind {
        TowerSound::Open => strike_recipe(&OPEN_STRIKES, 1.05, 0.28, Where::Here),
        TowerSound::Tab => strike_recipe(&TAB_STRIKES, 0.48, 0.15, Where::Here),
        TowerSound::Select => strike_recipe(&SELECT_STRIKES, 0.48, 0.17, Where::Here),
        TowerSound::DraftStart => strike_recipe(&DRAFT_STRIKES, 1.25, 0.36, Where::Ending),
        TowerSound::Buy => strike_recipe(&BUY_STRIKES, 0.85, 0.33, Where::Here),
        TowerSound::Sell => strike_recipe(&SELL_STRIKES, 0.85, 0.29, Where::Here),
        TowerSound::Reroll => strike_recipe(&REROLL_STRIKES, 0.92, 0.27, Where::Here),
        TowerSound::Freeze => strike_recipe(&FREEZE_STRIKES, 0.88, 0.25, Where::Here),
        TowerSound::Thaw => strike_recipe(&THAW_STRIKES, 0.88, 0.23, Where::Here),
        TowerSound::Equip => Recipe {
            strikes: &EQUIP_STRIKES,
            thuds: &EQUIP_THUDS,
            len: 0.82,
            peak: 0.28,
            room: Where::Here,
        },
        TowerSound::Store => strike_recipe(&STORE_STRIKES, 0.78, 0.21, Where::Here),
        TowerSound::Fusion => strike_recipe(&FUSION_STRIKES, 1.45, 0.42, Where::Ending),
        TowerSound::RewardReveal => {
            strike_recipe(&REWARD_REVEAL_STRIKES, 1.25, 0.34, Where::Ending)
        }
        TowerSound::RewardClaim => strike_recipe(&REWARD_CLAIM_STRIKES, 0.95, 0.31, Where::Here),
        TowerSound::StageEnter => strike_recipe(&STAGE_ENTER_STRIKES, 1.40, 0.38, Where::Ending),
        TowerSound::DrawerOpen => strike_recipe(&DRAWER_STRIKES, 0.62, 0.18, Where::Here),
        TowerSound::ItemUse => strike_recipe(&ITEM_USE_STRIKES, 0.78, 0.26, Where::Here),
        TowerSound::Refused => Recipe {
            strikes: &[],
            thuds: &REFUSED_THUDS,
            len: 0.50,
            peak: 0.24,
            room: Where::Here,
        },
    }
}

/// Synthesises one [`TowerSound`] stereo buffer on demand and caches its handle.
pub(crate) fn tower_voice(kind: TowerSound) -> AudioSource {
    source(wav(&render(&recipe(kind))))
}

#[derive(Resource, Default)]
struct CachedTowerVoices(Vec<(TowerSound, Handle<AudioSource>)>);

pub(crate) fn install(app: &mut App) {
    if app.world().contains_resource::<CachedTowerVoices>() {
        return;
    }
    app.init_resource::<CachedTowerVoices>()
        .add_message::<TowerSound>()
        .add_systems(PostUpdate, play_tower_sounds);
}

fn play_tower_sounds(
    mut commands: Commands,
    mut events: MessageReader<TowerSound>,
    cache: Option<ResMut<CachedTowerVoices>>,
    sources: Option<ResMut<Assets<AudioSource>>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    if events.is_empty() {
        return;
    }
    let requested: Vec<TowerSound> = events.read().copied().collect();
    let (Some(mut cache), Some(mut sources)) = (cache, sources) else {
        return;
    };
    let level = prefs.map_or_else(baylee_client_core::cue::Loudness::default, |p| {
        p.all().sound
    });
    let mix = settings.as_ref().map_or(1.0, |s| {
        s.audio.effects_gain(crate::quality::focused(&windows))
    });
    if !level.audible() || mix <= 0.0 {
        return;
    }
    // Deduplicate identical cues on the same frame and cap simultaneous voices at 2
    // so rapid actions never stack into digital clipping.
    let mut played_this_frame: Vec<TowerSound> = Vec::with_capacity(2);
    for kind in requested {
        if played_this_frame.contains(&kind) {
            continue;
        }
        played_this_frame.push(kind);
        let handle = if let Some((_, handle)) = cache.0.iter().find(|(k, _)| *k == kind) {
            handle.clone()
        } else {
            let handle = sources.add(tower_voice(kind));
            cache.0.push((kind, handle.clone()));
            handle
        };
        commands.spawn((
            AudioPlayer::new(handle),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(level.gain() * mix)),
        ));
        if played_this_frame.len() >= 2 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tower_sound_synthesises_clean_bounded_stereo_pcm() {
        for kind in TowerSound::ALL
            .into_iter()
            .chain(std::iter::once(TowerSound::Refused))
        {
            let pcm = render(&recipe(kind));
            assert!(!pcm.is_empty(), "{kind:?} produced empty buffer");
            assert_eq!(pcm.len() % 2, 0, "{kind:?} must be interleaved stereo");
            let peak = pcm.iter().fold(0.0_f32, |acc, s| acc.max(s.abs()));
            assert!(
                pcm.iter().all(|s| s.is_finite()),
                "{kind:?} produced non-finite sample"
            );
            assert!(
                peak > 0.05 && peak <= 0.80,
                "{kind:?} peak {peak} out of safe acoustic range"
            );
            let tail = pcm[pcm.len() - 64..]
                .iter()
                .fold(0.0_f32, |acc, s| acc.max(s.abs()));
            assert!(
                tail < 0.02,
                "{kind:?} tail {tail} clipped at end of buffer (len too short)"
            );
        }
    }
}
