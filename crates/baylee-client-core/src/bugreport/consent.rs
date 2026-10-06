//! What the player allows a report to carry, remembered on this device.
//!
//! One switch per [`Category`], and one more for crash reports, which the
//! client sends by itself and therefore must have been told it may. Every
//! switch starts off: a player who never opened the form has allowed
//! nothing, and a crash report is asked about once ([`CrashConsent::Unasked`])
//! rather than assumed. The client keeps this in its per-device settings
//! file, so un-ticking a box (or answering "don't send") is the revocation.
//!
//! A game's record is different again ([`RecordConsent`]): it holds every
//! seat's cards, hidden ones too, so a standing yes would be a standing
//! leak. The only standing answer is "never"; the yes is given per report,
//! in the form, and nothing here can keep it.

use serde::{Deserialize, Serialize};

use crate::i18n::Phrase;

/// One kind of data a report may carry, one box in the form.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum Category {
    /// Platform, graphics adapter, window size, language.
    System,
    /// The seat's view of the board, the open question, what was selected.
    Game,
    /// The seat's own game log, other players' names replaced.
    Log,
    /// A summary of the settings and preferences.
    Settings,
    /// A picture of the window.
    Screenshot,
}

impl Category {
    /// Every category, in the order the form lists them.
    pub const ALL: [Self; 5] = [
        Self::System,
        Self::Game,
        Self::Log,
        Self::Settings,
        Self::Screenshot,
    ];

    /// The box's label.
    #[must_use]
    pub fn phrase(self) -> Phrase {
        match self {
            Self::System => Phrase::ReportCatSystem,
            Self::Game => Phrase::ReportCatGame,
            Self::Log => Phrase::ReportCatLog,
            Self::Settings => Phrase::ReportCatSettings,
            Self::Screenshot => Phrase::ReportCatScreenshot,
        }
    }

    /// The line under the label saying what it holds, on a client that
    /// can (`pictures`) or cannot take a picture of its window.
    ///
    /// A browser build cannot, and a box promising "the window as it was"
    /// there would be a promise the report never keeps.
    #[must_use]
    pub fn hint_where(self, pictures: bool) -> Phrase {
        match self {
            Self::Screenshot if !pictures => Phrase::ReportCatScreenshotHintWeb,
            _ => self.hint(),
        }
    }

    /// What the label adds when there is nothing to send under it: that
    /// there is nothing, or, for the screenshot on a client that takes no
    /// pictures, why.
    #[must_use]
    pub fn nothing_where(self, pictures: bool) -> Phrase {
        match self {
            Self::Screenshot if !pictures => Phrase::ReportCatNoShotOnWeb,
            _ => Phrase::ReportCatNothing,
        }
    }

    /// The line under the label saying what it holds.
    #[must_use]
    pub fn hint(self) -> Phrase {
        match self {
            Self::System => Phrase::ReportCatSystemHint,
            Self::Game => Phrase::ReportCatGameHint,
            Self::Log => Phrase::ReportCatLogHint,
            Self::Settings => Phrase::ReportCatSettingsHint,
            Self::Screenshot => Phrase::ReportCatScreenshotHint,
        }
    }
}

/// Whether the client may send a crash report by itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashConsent {
    /// Never asked: the next crash asks, once.
    #[default]
    Unasked,
    /// Send each one to the gateway, without asking again.
    Send,
    /// Never send one.
    Never,
}

/// Whether the form offers to attach the record of a game this device
/// hosted (`docs/feedback.md`, "A local game's record").
///
/// Two answers, and neither of them is "send": a record leaves the machine
/// only on a box ticked for the one report it goes with, unticked again at
/// every opening of the form ([`crate::bugreport::ReportForm::send_record`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordConsent {
    /// Offer the box, unticked, whenever there is a record.
    #[default]
    Ask,
    /// Never offer it, and never attach one.
    Never,
}

/// The player's standing answers about reports, on this device.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one independent box per category, each ticked on its own; a set \
              would read the same and serialise less plainly in a settings file"
)]
pub struct Consent {
    /// [`Category::System`].
    pub system: bool,
    /// [`Category::Game`].
    pub game: bool,
    /// [`Category::Log`].
    pub log: bool,
    /// [`Category::Settings`].
    pub settings: bool,
    /// [`Category::Screenshot`].
    pub screenshot: bool,
    /// Crash reports.
    pub crashes: CrashConsent,
    /// Whether a local game's record is offered at all.
    pub record: RecordConsent,
}

impl Consent {
    /// Every box ticked; crash reports untouched. For tests and previews.
    #[must_use]
    pub fn everything() -> Self {
        let mut all = Self::default();
        for category in Category::ALL {
            all.set(category, true);
        }
        all
    }

    /// Whether `category` may be sent.
    #[must_use]
    pub fn allows(&self, category: Category) -> bool {
        match category {
            Category::System => self.system,
            Category::Game => self.game,
            Category::Log => self.log,
            Category::Settings => self.settings,
            Category::Screenshot => self.screenshot,
        }
    }

    /// Ticks or clears `category`.
    pub fn set(&mut self, category: Category, allowed: bool) {
        *match category {
            Category::System => &mut self.system,
            Category::Game => &mut self.game,
            Category::Log => &mut self.log,
            Category::Settings => &mut self.settings,
            Category::Screenshot => &mut self.screenshot,
        } = allowed;
    }

    /// Flips `category`.
    pub fn toggle(&mut self, category: Category) {
        self.set(category, !self.allows(category));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A device that was never asked has allowed nothing, and a settings
    /// file written before this field existed reads as that device.
    #[test]
    fn nothing_is_allowed_until_it_is_ticked() {
        let consent = Consent::default();
        assert!(Category::ALL.iter().all(|c| !consent.allows(*c)));
        assert_eq!(consent.crashes, CrashConsent::Unasked);
        let read: Consent = serde_json::from_str("{}").expect("an empty object reads");
        assert_eq!(read, consent);
    }

    /// A settings file from before the record's answer offers the box, and
    /// "never" is kept; there is no third answer for a file to hold, so a
    /// yes cannot be remembered by hand-editing one either.
    #[test]
    fn the_only_standing_answer_about_a_record_is_never() {
        let old: Consent = serde_json::from_str(r#"{"system":true,"crashes":"send"}"#)
            .expect("a file from before reads");
        assert_eq!(old.record, RecordConsent::Ask);
        let never = Consent {
            record: RecordConsent::Never,
            ..Consent::default()
        };
        let text = serde_json::to_string(&never).expect("json");
        assert!(text.contains(r#""record":"never""#), "{text}");
        assert_eq!(
            serde_json::from_str::<Consent>(&text).expect("reads"),
            never
        );
        for yes in [
            r#"{"record":"send"}"#,
            r#"{"record":true}"#,
            r#"{"record":"always"}"#,
        ] {
            assert!(
                serde_json::from_str::<Consent>(yes).is_err(),
                "{yes} was read as an answer"
            );
        }
    }

    /// A client that takes no picture says so at the screenshot box, under
    /// it and beside it, and says nothing different at any other box.
    #[test]
    fn a_client_without_pictures_says_so_at_the_screenshot_box_only() {
        for category in Category::ALL {
            assert_eq!(category.hint_where(true), category.hint());
            assert_eq!(category.nothing_where(true), Phrase::ReportCatNothing);
            if category == Category::Screenshot {
                assert_eq!(
                    category.hint_where(false),
                    Phrase::ReportCatScreenshotHintWeb
                );
                assert_eq!(category.nothing_where(false), Phrase::ReportCatNoShotOnWeb);
            } else {
                assert_eq!(category.hint_where(false), category.hint());
                assert_eq!(category.nothing_where(false), Phrase::ReportCatNothing);
            }
        }
    }

    /// Each box is its own switch, and un-ticking one is remembered as
    /// firmly as ticking it: that is the revocation.
    #[test]
    fn each_box_is_its_own_switch_and_survives_a_round_trip() {
        for category in Category::ALL {
            let mut consent = Consent::default();
            consent.toggle(category);
            for other in Category::ALL {
                assert_eq!(consent.allows(other), other == category);
            }
            consent.crashes = CrashConsent::Never;
            let back: Consent =
                serde_json::from_str(&serde_json::to_string(&consent).expect("json"))
                    .expect("reads back");
            assert_eq!(back, consent);
            consent.toggle(category);
            assert!(
                !consent.allows(category),
                "{category:?} could not be revoked"
            );
        }
    }
}
