//! The eight musical situations in every suite.
use crate::music::{Scene, ScoreRequest};

/// A complete arrangement within the selected suite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Movement {
    /// The title's broad statement, with every instrument group.
    #[default]
    Title,
    /// A small, relaxed Dorian tavern ensemble.
    Lobby,
    /// Plucked melody and bowed accompaniment at the table.
    Standard,
    /// Close intervals and rhythmic bowed strings.
    Combat,
    /// Wide intervals and foreground brass and strings.
    Endgame,
    /// An ascending attention cue followed by a bright celebration.
    Victory,
    /// A descending attention cue followed by a slow lament.
    Defeat,
    /// An augmented cue followed by major/minor ambiguity.
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
            ..ScoreRequest::default()
        }
    }

    pub(super) const fn ending(self) -> bool {
        matches!(self, Self::Victory | Self::Defeat | Self::Draw)
    }

    pub(super) fn wanted(request: ScoreRequest, previous: Self) -> Self {
        match request.scene {
            Scene::FrontDoor => Self::Title,
            Scene::Lobby | Scene::Build => Self::Lobby,
            Scene::Opening => Self::Standard,
            Scene::Victory => Self::Victory,
            Scene::Defeat => Self::Defeat,
            Scene::Draw => Self::Draw,
            Scene::Table => {
                // Hysteresis prevents repeated changes near an intensity threshold.
                let threshold = if previous == Self::Endgame {
                    0.58
                } else {
                    0.78
                };
                if request.tension >= threshold || request.about_to_lose || request.lethal {
                    Self::Endgame
                } else if request.combat {
                    Self::Combat
                } else {
                    Self::Standard
                }
            }
        }
    }
}
