//! The settings screen's map (the shell design, `DESIGN-v5` §12; WP5): its
//! ten sections, every row in them, where each row is kept, and the search
//! over them.
//!
//! Pure, so the screen and its tests read the same list: the screen draws a
//! section's rows in this order with this storage tag, the search lists rows
//! with their section as a crumb and jumps to them, and a test holds every
//! row to a section, a tag and words that find it.
//!
//! A row is drawn only where its mechanism exists (principle 5: a control
//! that does not work is not shown) — [`Builds`] says which: no updater on
//! a browser or a phone, no frame pacing or window mode in a browser, no
//! language-model seat but on a desktop.

use crate::i18n::{Lang, Phrase};

/// One of the ten sections, in the sidebar's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Section {
    /// The device's graphics, and the account's three rows about the table.
    #[default]
    Graphics,
    /// Volumes, and the account's table sounds.
    Audio,
    /// Language, text size, the preview.
    Display,
    /// Keys: the shell's and the table's.
    Controls,
    /// Automation, where to stop, ability answers.
    Gameplay,
    /// The handle, signing out, deleting the account.
    Account,
    /// The gateway: where, which version, diagnostics.
    Network,
    /// The language-model seat's profiles (this machine's).
    LanguageModels,
    /// The updater (desktop builds).
    Updates,
    /// What is kept and sent; report consent.
    Privacy,
}

impl Section {
    /// Every section, in the sidebar's order.
    pub const ALL: [Self; 10] = [
        Self::Graphics,
        Self::Audio,
        Self::Display,
        Self::Controls,
        Self::Gameplay,
        Self::Account,
        Self::Network,
        Self::LanguageModels,
        Self::Updates,
        Self::Privacy,
    ];

    /// The section's name on a chip (the narrow chip row), within a
    /// chip's fourteen characters.
    #[must_use]
    pub const fn short(self) -> Phrase {
        match self {
            Self::Display => Phrase::SectionDisplayShort,
            Self::Network => Phrase::SectionNetworkShort,
            Self::LanguageModels => Phrase::SectionModelsShort,
            Self::Privacy => Phrase::SectionPrivacyShort,
            other => other.name(),
        }
    }

    /// Its name in the sidebar and over its panel.
    #[must_use]
    pub const fn name(self) -> Phrase {
        match self {
            Self::Graphics => Phrase::SectionGraphics,
            Self::Audio => Phrase::SectionAudio,
            Self::Display => Phrase::SectionDisplay,
            Self::Controls => Phrase::SectionControls,
            Self::Gameplay => Phrase::SectionGameplay,
            Self::Account => Phrase::SectionAccount,
            Self::Network => Phrase::SectionNetwork,
            Self::LanguageModels => Phrase::SectionLanguageModels,
            Self::Updates => Phrase::SectionUpdates,
            Self::Privacy => Phrase::SectionPrivacy,
        }
    }

    /// Where its rows are mostly kept, for the badge beside its title (a
    /// row with another store says so on its own tag).
    #[must_use]
    pub const fn scope(self) -> Scope {
        match self {
            // The gateways are this device's list (`ClientSettings`), not
            // the account's: a phone and a desktop keep their own.
            Self::Graphics
            | Self::Audio
            | Self::Display
            | Self::Network
            | Self::Updates
            | Self::Privacy => Scope::Device,
            Self::Controls | Self::Gameplay | Self::Account => Scope::Account,
            Self::LanguageModels => Scope::Machine,
        }
    }

    /// Whether a build has this section at all. A browser or a phone has
    /// no updater; the language models are one read-only line there.
    #[must_use]
    pub const fn offered(self, builds: Builds) -> bool {
        match self {
            Self::Updates => builds.desktop,
            _ => true,
        }
    }

    /// The section whose name starts with `letters` (the sidebar's
    /// type-ahead, letters only), among those offered.
    #[must_use]
    pub fn typed(letters: &str, lang: Lang, builds: Builds) -> Option<Self> {
        let want = fold(letters);
        if want.is_empty() {
            return None;
        }
        Self::ALL
            .into_iter()
            .filter(|s| s.offered(builds))
            .find(|s| fold(s.name().text(lang)).starts_with(&want))
    }
}

/// Where a row is kept: the tag beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// This device (`ClientSettings`, `update.json`): "this device".
    Device,
    /// The account (`Preferences` over `/settings`): "your account".
    Account,
    /// This machine's `llm-seat.json`: "this machine".
    Machine,
}

impl Scope {
    /// The tag's words.
    #[must_use]
    pub const fn tag(self) -> Phrase {
        match self {
            Self::Device => Phrase::TagDevice,
            Self::Account => Phrase::TagAccount,
            Self::Machine => Phrase::TagMachine,
        }
    }
}

/// Which mechanisms the running build has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Builds {
    /// A desktop build: the updater, the window, the language-model seat.
    pub desktop: bool,
    /// A browser build: the browser presents frames and owns the window.
    pub web: bool,
    /// A phone: anti-aliasing locked off (`docs/mobile.md`).
    pub phone: bool,
}

impl Builds {
    /// A desktop build.
    pub const DESKTOP: Self = Self {
        desktop: true,
        web: false,
        phone: false,
    };
    /// A browser build.
    pub const WEB: Self = Self {
        desktop: false,
        web: true,
        phone: false,
    };
    /// A phone build.
    pub const PHONE: Self = Self {
        desktop: false,
        web: false,
        phone: true,
    };
}

/// Every row the screen draws, by name (also its press's and its search
/// result's name).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Row {
    // Graphics (device)
    /// The preset: Low · Medium · High · Ultra, Custom derived.
    Preset,
    /// Windowed · Borderless · Fullscreen, with the 15-s revert.
    DisplayMode,
    /// Which monitor (shown with more than one).
    Monitor,
    /// Edge smoothing.
    AntiAliasing,
    /// Whether frames wait for the display.
    VSync,
    /// Frames per second with the focus.
    FrameLimit,
    /// Frames per second without it.
    BackgroundLimit,
    /// The frame-time counter in a corner.
    ShowFrameRate,
    /// The painting behind the panels: Painting · Dimmed · Plain.
    Backdrop,
    /// The drifting light and dust (capped by the account's Atmosphere).
    Ambient,
    /// How steeply a table of three or more seats is shot (DESIGN-v7 D20).
    TableLean,
    /// Where the camera stands on a visit (DESIGN-v7 D21).
    VisitCamera,
    /// How the seats are placed by default (DESIGN-v8 §2.4).
    Arrangement,
    /// Whether the table follows the turn (DESIGN-v8 §1.1, D25).
    FollowTurn,
    /// The arrangement remembered per seat count (DESIGN-v8 §2.6, D24).
    ArrangementBySeats,
    // Graphics (account)
    /// The account's atmosphere: the ceiling of Ambient.
    Atmosphere,
    /// The account's reduce-motion switch.
    HoldStill,
    /// The account's sky.
    Sky,
    // Audio
    /// Every sound.
    Master,
    /// The music.
    Music,
    /// The table's cues, under the account's ceiling.
    Effects,
    /// Silent behind other windows.
    MuteUnfocused,
    /// The priority cue's own switch (DESIGN-v7 D22).
    PriorityCue,
    /// The account's table sounds: Off · Half · Full.
    TableSounds,
    // Display & Interface
    /// The interface's language (and the card text's).
    Language,
    /// The five text steps.
    TextSize,
    /// The card preview's size.
    PreviewSize,
    /// The text face instead of the print.
    TextFace,
    // Controls
    /// The shell's shortcuts.
    ShellKeys,
    /// The table's keys.
    TableKeys,
    // Gameplay
    /// The four automation rules.
    Automation,
    /// Where to stop: the rails and their presets.
    Stops,
    /// The answers given to abilities.
    AbilityAnswers,
    // Account
    /// The handle, and copying it.
    Handle,
    /// Signing out.
    SignOut,
    /// Deleting the account.
    DeleteAccount,
    // Network & Gateway
    /// The gateway: name, address, versions.
    Gateway,
    /// Back to the gateway list.
    SwitchGateway,
    /// The connection's state, as text to copy.
    Diagnostics,
    // Language models
    /// The seat's profiles and their caps.
    Profiles,
    // Updates
    /// Ask for updates on its own.
    CheckAutomatically,
    /// Install them on its own.
    InstallAutomatically,
    /// Ask now; what was found; why it does not install.
    CheckNow,
    // Privacy & Data
    /// What the gateway keeps, and what this client sends on its own.
    WhatIsKept,
    /// What a report may carry.
    ReportConsent,
    /// Crash reports.
    CrashReports,
    /// The report form.
    ReportProblem,
}

/// One row of the map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowDef {
    /// The row.
    pub row: Row,
    /// Its section.
    pub section: Section,
    /// Where it is kept.
    pub scope: Scope,
    /// Its label.
    pub label: Phrase,
    /// What it does, with the cost or the effect in words.
    pub help: Phrase,
}

const fn def(row: Row, section: Section, scope: Scope, label: Phrase, help: Phrase) -> RowDef {
    RowDef {
        row,
        section,
        scope,
        label,
        help,
    }
}

use Phrase as P;
use Scope::{Account as A, Device as D, Machine as M};
use Section as S;

/// Every row, in each section's order.
pub const ROWS: &[RowDef] = &[
    def(Row::Preset, S::Graphics, D, P::RowPreset, P::HelpPreset),
    def(
        Row::DisplayMode,
        S::Graphics,
        D,
        P::RowDisplayMode,
        P::HelpDisplayMode,
    ),
    def(Row::Monitor, S::Graphics, D, P::RowMonitor, P::HelpMonitor),
    def(
        Row::AntiAliasing,
        S::Graphics,
        D,
        P::AntiAliasing,
        P::HelpAntiAliasing,
    ),
    def(Row::VSync, S::Graphics, D, P::VSync, P::HelpVSync),
    def(
        Row::FrameLimit,
        S::Graphics,
        D,
        P::FrameLimit,
        P::HelpFrameLimit,
    ),
    def(
        Row::BackgroundLimit,
        S::Graphics,
        D,
        P::BackgroundFrames,
        P::HelpBackgroundLimit,
    ),
    def(
        Row::ShowFrameRate,
        S::Graphics,
        D,
        P::RowShowFrameRate,
        P::HelpShowFrameRate,
    ),
    def(
        Row::Backdrop,
        S::Graphics,
        D,
        P::RowBackdrop,
        P::HelpBackdrop,
    ),
    def(
        Row::Ambient,
        S::Graphics,
        D,
        P::AmbientEffects,
        P::HelpAmbient,
    ),
    def(
        Row::TableLean,
        S::Graphics,
        D,
        P::RowTableLean,
        P::HelpTableLean,
    ),
    def(
        Row::VisitCamera,
        S::Graphics,
        D,
        P::RowVisitCamera,
        P::HelpVisitCamera,
    ),
    def(
        Row::Arrangement,
        S::Graphics,
        D,
        P::RowArrangement,
        P::HelpArrangement,
    ),
    def(
        Row::FollowTurn,
        S::Graphics,
        D,
        P::RowFollowTurn,
        P::HelpFollowTurn,
    ),
    def(
        Row::ArrangementBySeats,
        S::Graphics,
        D,
        P::RowArrangementBySeats,
        P::HelpArrangementBySeats,
    ),
    def(
        Row::Atmosphere,
        S::Graphics,
        A,
        P::Atmosphere,
        P::HelpAtmosphere,
    ),
    def(
        Row::HoldStill,
        S::Graphics,
        A,
        P::RowHoldStill,
        P::HelpHoldStill,
    ),
    def(Row::Sky, S::Graphics, A, P::Sky, P::HelpSky),
    def(Row::Master, S::Audio, D, P::MasterVolume, P::HelpMaster),
    def(Row::Music, S::Audio, D, P::RowMusic, P::HelpMusic),
    def(Row::Effects, S::Audio, D, P::EffectsVolume, P::HelpEffects),
    def(
        Row::MuteUnfocused,
        S::Audio,
        D,
        P::MuteInBackground,
        P::HelpMuteUnfocused,
    ),
    def(
        Row::PriorityCue,
        S::Audio,
        D,
        P::RowPriorityCue,
        P::HelpPriorityCue,
    ),
    def(
        Row::TableSounds,
        S::Audio,
        A,
        P::RowTableSounds,
        P::HelpTableSounds,
    ),
    def(Row::Language, S::Display, D, P::Language, P::HelpLanguage),
    def(
        Row::TextSize,
        S::Display,
        D,
        P::RowTextSize,
        P::HelpTextSize,
    ),
    def(
        Row::PreviewSize,
        S::Display,
        D,
        P::RowPreviewSize,
        P::HelpPreviewSize,
    ),
    def(
        Row::TextFace,
        S::Display,
        D,
        P::RowTextFace,
        P::HelpTextFace,
    ),
    def(
        Row::ShellKeys,
        S::Controls,
        A,
        P::RowShellKeys,
        P::HelpShellKeys,
    ),
    def(
        Row::TableKeys,
        S::Controls,
        A,
        P::RowTableKeys,
        P::HelpTableKeys,
    ),
    def(
        Row::Automation,
        S::Gameplay,
        A,
        P::Automation,
        P::HelpAutomation,
    ),
    def(Row::Stops, S::Gameplay, A, P::WhereToStop, P::RailExplain),
    def(
        Row::AbilityAnswers,
        S::Gameplay,
        A,
        P::RowAbilityAnswers,
        P::HelpAbilityAnswers,
    ),
    def(Row::Handle, S::Account, A, P::RowHandle, P::HelpHandle),
    def(Row::SignOut, S::Account, A, P::RowSignOut, P::HelpSignOut),
    def(
        Row::DeleteAccount,
        S::Account,
        A,
        P::DeleteAccount,
        P::HelpDeleteAccount,
    ),
    def(Row::Gateway, S::Network, D, P::RowGateway, P::HelpGateway),
    def(
        Row::SwitchGateway,
        S::Network,
        D,
        P::ShellSwitchGateway,
        P::HelpSwitchGateway,
    ),
    def(
        Row::Diagnostics,
        S::Network,
        D,
        P::RowDiagnostics,
        P::HelpDiagnostics,
    ),
    def(
        Row::Profiles,
        S::LanguageModels,
        M,
        P::RowProfiles,
        P::SeatPanelAbout,
    ),
    def(
        Row::CheckAutomatically,
        S::Updates,
        D,
        P::RowCheckUpdates,
        P::HelpCheckUpdates,
    ),
    def(
        Row::InstallAutomatically,
        S::Updates,
        D,
        P::RowInstallUpdates,
        P::HelpInstallUpdates,
    ),
    def(
        Row::CheckNow,
        S::Updates,
        D,
        P::RowCheckNow,
        P::HelpCheckNow,
    ),
    def(
        Row::WhatIsKept,
        S::Privacy,
        D,
        P::RowWhatIsKept,
        P::HelpWhatIsKept,
    ),
    def(
        Row::ReportConsent,
        S::Privacy,
        D,
        P::RowReportConsent,
        P::HelpReportConsent,
    ),
    def(
        Row::CrashReports,
        S::Privacy,
        D,
        P::RowCrashReports,
        P::HelpCrashReports,
    ),
    def(
        Row::ReportProblem,
        S::Privacy,
        D,
        P::ShellReportProblem,
        P::HelpReportProblem,
    ),
];

/// The map's entry for `row`.
#[must_use]
pub fn of(row: Row) -> &'static RowDef {
    ROWS.iter().find(|d| d.row == row).unwrap_or(&ROWS[0])
}

/// Whether a build draws `row` (principle 5: no control without its
/// mechanism). `monitors` is how many the window can be put on.
#[must_use]
pub fn offered(row: Row, builds: Builds, monitors: usize) -> bool {
    match row {
        // The browser presents frames and owns the window.
        Row::DisplayMode | Row::VSync | Row::FrameLimit | Row::BackgroundLimit => {
            !builds.web && !builds.phone
        }
        Row::Monitor => builds.desktop && monitors > 1,
        Row::CheckAutomatically | Row::InstallAutomatically | Row::CheckNow => builds.desktop,
        _ => of(row).section.offered(builds),
    }
}

/// A section's rows a build draws, in order.
pub fn rows_of(
    section: Section,
    builds: Builds,
    monitors: usize,
) -> impl Iterator<Item = &'static RowDef> {
    ROWS.iter()
        .filter(move |d| d.section == section && offered(d.row, builds, monitors))
}

/// Lower-case, umlauts folded, so a search finds "Schrift" in "schrift"
/// and "Groesse" in "Größe".
fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars().flat_map(char::to_lowercase) {
        match c {
            'ä' => out.push_str("ae"),
            'ö' => out.push_str("oe"),
            'ü' => out.push_str("ue"),
            'ß' => out.push_str("ss"),
            c => out.push(c),
        }
    }
    out
}

/// The rows whose label, help or section holds every word of `query`, in
/// the map's order; nothing for a blank query.
#[must_use]
pub fn search(query: &str, lang: Lang, builds: Builds, monitors: usize) -> Vec<Row> {
    let words: Vec<String> = query.split_whitespace().map(fold).collect();
    if words.is_empty() {
        return Vec::new();
    }
    ROWS.iter()
        .filter(|d| offered(d.row, builds, monitors))
        .filter(|d| {
            let hay = fold(&format!(
                "{} {} {}",
                d.label.text(lang),
                d.help.text(lang),
                d.section.name().text(lang)
            ));
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .map(|d| d.row)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_section_has_rows_and_every_row_one_entry() {
        for section in Section::ALL {
            assert!(
                rows_of(section, Builds::DESKTOP, 2).next().is_some(),
                "{section:?} is empty"
            );
        }
        let mut rows: Vec<Row> = ROWS.iter().map(|d| d.row).collect();
        let n = rows.len();
        rows.sort();
        rows.dedup();
        assert_eq!(rows.len(), n, "a row twice in the map");
        // In each section's order: a section's rows stand together.
        let sections: Vec<Section> = ROWS.iter().map(|d| d.section).collect();
        let mut sorted = sections.clone();
        sorted.sort();
        assert_eq!(sections, sorted);
    }

    /// The account's rows in Graphics and Audio say so; the rest of those
    /// sections is the device's (C3-4, M4-3: a preset writes device rows).
    #[test]
    fn the_account_rows_are_tagged_as_the_account_s() {
        for row in [Row::Atmosphere, Row::HoldStill, Row::Sky, Row::TableSounds] {
            assert_eq!(of(row).scope, Scope::Account, "{row:?}");
        }
        for row in [
            Row::Preset,
            Row::AntiAliasing,
            Row::Ambient,
            Row::Master,
            Row::Effects,
            Row::TableLean,
            Row::VisitCamera,
            Row::Arrangement,
            Row::FollowTurn,
            Row::ArrangementBySeats,
            Row::PriorityCue,
        ] {
            assert_eq!(of(row).scope, Scope::Device, "{row:?}");
        }
        assert_eq!(of(Row::Profiles).scope, Scope::Machine);
    }

    /// No updater in a browser or on a phone; the browser presents frames
    /// itself; a monitor row only with more than one monitor.
    #[test]
    fn a_build_draws_only_the_rows_it_has_a_mechanism_for() {
        assert!(!Section::Updates.offered(Builds::WEB));
        assert!(!Section::Updates.offered(Builds::PHONE));
        assert!(Section::Updates.offered(Builds::DESKTOP));
        for row in [
            Row::VSync,
            Row::FrameLimit,
            Row::DisplayMode,
            Row::BackgroundLimit,
        ] {
            assert!(!offered(row, Builds::WEB, 1), "{row:?} on the web");
            assert!(offered(row, Builds::DESKTOP, 1), "{row:?} on a desktop");
        }
        assert!(!offered(Row::Monitor, Builds::DESKTOP, 1));
        assert!(offered(Row::Monitor, Builds::DESKTOP, 2));
        assert_eq!(rows_of(Section::Updates, Builds::WEB, 1).count(), 0);
        // One rule for sound, and no interface slider: no lobby cue exists
        // (S4-4).
        let audio: Vec<Row> = rows_of(Section::Audio, Builds::DESKTOP, 1)
            .map(|d| d.row)
            .collect();
        assert_eq!(
            audio,
            [
                Row::Master,
                Row::Music,
                Row::Effects,
                Row::MuteUnfocused,
                Row::PriorityCue,
                Row::TableSounds
            ]
        );
    }

    #[test]
    fn search_finds_rows_by_their_words_in_either_language() {
        let found = search("frame", Lang::En, Builds::DESKTOP, 1);
        assert!(
            found.contains(&Row::FrameLimit) && found.contains(&Row::ShowFrameRate),
            "{found:?}"
        );
        assert!(search("", Lang::En, Builds::DESKTOP, 1).is_empty());
        assert!(search("zzzz", Lang::En, Builds::DESKTOP, 1).is_empty());
        let german = search("textgroesse", Lang::De, Builds::DESKTOP, 1);
        assert_eq!(german, vec![Row::TextSize], "folded umlauts");
        // A section's name finds its rows.
        assert!(search("audio", Lang::En, Builds::DESKTOP, 1).contains(&Row::Master));
        // Rows a build does not draw are not found.
        assert!(search("vsync", Lang::En, Builds::WEB, 1).is_empty());
    }

    /// A chip holds fourteen characters (the kit's label budget): every
    /// section's chip name fits, in both languages.
    #[test]
    fn every_chip_name_fits_a_chip() {
        for section in Section::ALL {
            for lang in [Lang::En, Lang::De] {
                let words = section.short().text(lang);
                assert!(words.chars().count() <= 14, "{words}");
            }
        }
    }

    #[test]
    fn the_sidebar_type_ahead_takes_letters() {
        assert_eq!(
            Section::typed("co", Lang::En, Builds::DESKTOP),
            Some(Section::Controls)
        );
        assert_eq!(
            Section::typed("g", Lang::En, Builds::DESKTOP),
            Some(Section::Graphics)
        );
        assert_eq!(
            Section::typed("ga", Lang::En, Builds::DESKTOP),
            Some(Section::Gameplay)
        );
        assert_eq!(
            Section::typed("u", Lang::En, Builds::WEB),
            None,
            "no updater there"
        );
        assert_eq!(Section::typed("", Lang::En, Builds::DESKTOP), None);
    }
}
