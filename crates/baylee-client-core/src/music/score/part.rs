//! The eight musical situations in every suite.
use crate::music::{Scene, ScoreRequest};

/// A complete arrangement within the selected suite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Movement {
    /// Epic Baylee Signature Overture in D minor / D Dorian / D major at sign-in (`FrontDoor`).
    #[default]
    Title,
    /// Cozy Acoustic-Guitar-led public-domain Tavern suite for Lobby and Deckbuilder.
    Lobby,
    /// Calm ("Ruhig") chamber guitar, concert harp and strings at the table.
    Standard,
    /// Poised / tense rhythmic ostinato, spiccato strings and horn counterpoint.
    Combat,
    /// Epic ("Episch") full brass, timpani, soaring strings and celesta climax.
    Endgame,
    /// Radiant ascending D-major fanfare followed by triumphant celebration.
    Victory,
    /// Somber descending D-minor cue followed by a noble elegy.
    Defeat,
    /// Open-fifth D–A cue followed by contemplative D-Dorian modal suspension.
    Draw,
}

impl Movement {
    /// All eight arrangements, in audition order.
    pub const ALL: [Self; 8] = [
        Self::Title,
        Self::Lobby,
        Self::Standard,
        Self::Combat,
        Self::Endgame,
        Self::Victory,
        Self::Defeat,
        Self::Draw,
    ];

    /// Stable name used in WAV exports and the development controls.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Lobby => "lobby",
            Self::Standard => "standard",
            Self::Combat => "combat",
            Self::Endgame => "endgame",
            Self::Victory => "victory",
            Self::Defeat => "defeat",
            Self::Draw => "draw",
        }
    }

    /// A request that auditions this arrangement through the runtime conductor.
    #[must_use]
    pub fn request(self, theme: super::Theme) -> ScoreRequest {
        ScoreRequest {
            theme,
            scene: match self {
                Self::Title => Scene::FrontDoor,
                Self::Lobby => Scene::Lobby,
                Self::Victory => Scene::Victory,
                Self::Defeat => Scene::Defeat,
                Self::Draw => Scene::Draw,
                _ => Scene::Table,
            },
            combat: self == Self::Combat,
            low_life: self == Self::Endgame,
            tension: if self == Self::Endgame { 0.9 } else { 0.0 },
            stack_depth: if self == Self::Endgame { 4 } else { 0 },
            action_intensity: match self {
                Self::Endgame => 0.92,
                Self::Combat => 0.45,
                _ => 0.0,
            },
            ..ScoreRequest::default()
        }
    }

    pub(super) const fn ending(self) -> bool {
        matches!(self, Self::Victory | Self::Defeat | Self::Draw)
    }

    pub(super) const fn table(self) -> bool {
        matches!(self, Self::Standard | Self::Combat | Self::Endgame)
    }

    pub(super) fn wanted(request: ScoreRequest, previous: Self) -> Self {
        match request.scene {
            Scene::FrontDoor => Self::Title,
            // Opening holds a hushed harmonic bridge in Tune; table music never starts during loading.
            Scene::Lobby | Scene::Build | Scene::Opening => Self::Lobby,
            Scene::Victory => Self::Victory,
            Scene::Defeat => Self::Defeat,
            Scene::Draw => Self::Draw,
            Scene::Table => {
                let epic = request.epicness();
                // Hysteresis prevents repeated flips near an intensity or stack threshold.
                let endgame_threshold = if previous == Self::Endgame {
                    0.56
                } else {
                    0.75
                };
                let combat_threshold = if previous == Self::Combat { 0.28 } else { 0.44 };
                if request.tension >= endgame_threshold
                    || epic >= endgame_threshold
                    || request.stack_depth >= 4
                    || (request.stack_depth >= 3
                        && (request.big_spell || request.combat || request.low_life))
                    || request.about_to_lose
                    || request.lethal
                {
                    Self::Endgame
                } else if request.combat || request.stack_depth >= 2 || epic >= combat_threshold {
                    Self::Combat
                } else {
                    Self::Standard
                }
            }
        }
    }
}
