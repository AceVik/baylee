//! What a card says, before it is drawn: its mark, title, lines and
//! actions for each state of the notice.

use super::super::{Asked, Fetch, Progress, Shown, UpdateButton, UpdatePlace, Why};
use crate::shellkit::controls::Weight;
use crate::shellkit::tokens;
use baylee_client_core::i18n::{Lang, Phrase};
use bevy::prelude::*;

pub(super) const DOWNLOAD: char = '\u{f019}';
pub(super) const SHIELD: char = '\u{f3ed}';
pub(super) const BOX: char = '\u{f49e}';
pub(super) const DONE: char = '\u{f058}';
pub(super) const WARNING: char = '\u{f071}';
pub(super) const INFO: char = '\u{f05a}';
pub(super) const FOLDER: char = '\u{f07c}';
pub(super) const CLOSE: char = '\u{f00d}';

/// The line under the headline: the exact folder and error where the
/// installation's folder is read-only, the notice's own reason otherwise.
pub(super) fn reason_of(shown: &Shown, place: &UpdatePlace, lang: Lang) -> Option<String> {
    match (shown, &place.read_only) {
        (
            Shown::Available {
                why: Why::Folder, ..
            },
            Some((folder, error)),
        ) => Some(Phrase::UpdateWhyReadOnly.fill(lang, &[folder, error])),
        _ => shown.reason(lang).map(str::to_owned),
    }
}

/// Whether the notice offers the move: it is only a link because of where
/// the app lies, and a move is possible.
pub(super) fn offers_move(shown: &Shown, place: &UpdatePlace) -> bool {
    !place.moves.is_empty()
        && matches!(
            shown,
            Shown::Available {
                why: Why::Folder | Why::MoveApp,
                ..
            }
        )
}

/// What a card is drawn from.
pub(super) struct Card<'a> {
    pub(super) shown: Option<&'a Shown>,
    pub(super) place: &'a UpdatePlace,
    pub(super) retrying: Option<Asked>,
    pub(super) progress: Option<&'a Progress>,
}

/// One action of a card.
pub(super) struct Action {
    pub(super) label: String,
    pub(super) weight: Weight,
    pub(super) does: UpdateButton,
}

impl Action {
    fn new(label: impl Into<String>, weight: Weight, does: UpdateButton) -> Self {
        Self {
            label: label.into(),
            weight,
            does,
        }
    }

    /// The release notes, as the least of a card's actions.
    fn notes(lang: Lang, page: &str, weight: Weight) -> Self {
        Self::new(
            Phrase::ReleaseNotes.text(lang),
            weight,
            UpdateButton::Open(page.to_owned()),
        )
    }
}

/// What a card says, before it is drawn.
pub(super) struct Look {
    pub(super) mark: char,
    pub(super) accent: Color,
    pub(super) title: String,
    /// While fetching: which part. The bar and its lines stand under the
    /// title.
    pub(super) fetch: Option<Fetch>,
    pub(super) lines: Vec<String>,
    pub(super) actions: Vec<Action>,
    /// A close mark, which hides the card for the session.
    pub(super) close: bool,
}

impl Look {
    fn bare(mark: char, title: String) -> Self {
        Self {
            mark,
            accent: tokens::ACCENT,
            title,
            fetch: None,
            lines: Vec::new(),
            actions: Vec::new(),
            close: true,
        }
    }

    #[allow(clippy::too_many_lines)] // one arm per state, each short
    pub(super) fn of(card: &Card, lang: Lang) -> Self {
        // After a move, the old copy's question comes first: it is asked once.
        if let Some((now, old)) = &card.place.moved {
            let mut look = Self::bare(FOLDER, Phrase::UpdateMoved.fill(lang, &[now]));
            look.lines.push(Phrase::UpdateOldCopy.fill(lang, &[old]));
            look.lines.extend(card.place.failed.clone());
            look.actions = vec![
                Action::new(
                    Phrase::UpdateTrashOld.text(lang),
                    Weight::Secondary,
                    UpdateButton::TrashOld,
                ),
                Action::new(
                    Phrase::UpdateKeepOld.text(lang),
                    Weight::Ghost,
                    UpdateButton::KeepOld,
                ),
            ];
            return look;
        }
        let Some(shown) = card.shown else {
            return Self::bare(INFO, String::new());
        };
        match shown {
            Shown::Fetching { version, stage, .. } => {
                let mark = match stage {
                    Fetch::Downloading => DOWNLOAD,
                    Fetch::Verifying => SHIELD,
                    Fetch::Unpacking => BOX,
                };
                let mut look = Self::bare(mark, Phrase::UpdateCardTitle.fill(lang, &[version]));
                look.fetch = Some(*stage);
                look
            }
            Shown::Ready { version, .. } => {
                let mut look = Self::bare(DONE, Phrase::UpdateCardReady.fill(lang, &[version]));
                look.lines
                    .push(Phrase::UpdateRestartOffer.text(lang).to_owned());
                look.lines.extend(card.place.failed.clone());
                // Two actions and no third: at the card's width the release
                // notes would wrap onto a row of their own, and they are
                // what the "Updated to" card after the restart offers (and
                // the table menu's line, all along).
                look.actions = vec![
                    Action::new(
                        Phrase::UpdateRestartNow.text(lang),
                        Weight::Primary,
                        UpdateButton::RestartNow,
                    ),
                    Action::new(
                        Phrase::UpdateLater.text(lang),
                        Weight::Secondary,
                        UpdateButton::Later,
                    ),
                ];
                // "Later" is its way out.
                look.close = false;
                look
            }
            Shown::Failed {
                version,
                page,
                reason,
            } => {
                let mut look = Self::bare(WARNING, Phrase::UpdateCardTitle.fill(lang, &[version]));
                look.accent = tokens::DANGER;
                look.lines.push(match card.retrying {
                    Some(Asked::Checking) => Phrase::UpdateChecking.text(lang).to_owned(),
                    Some(Asked::Failed) => Phrase::UpdateCheckFailed.text(lang).to_owned(),
                    _ => Phrase::UpdateFailedWhy.fill(lang, &[reason]),
                });
                look.actions = vec![
                    Action::new(
                        Phrase::UpdateTryAgain.text(lang),
                        Weight::Secondary,
                        UpdateButton::TryAgain,
                    ),
                    Action::notes(lang, page, Weight::Ghost),
                ];
                look
            }
            Shown::Available { version, page, .. } => {
                let mut look = Self::bare(INFO, Phrase::UpdateCardAvailable.fill(lang, &[version]));
                look.lines.extend(reason_of(shown, card.place, lang));
                let moves = offers_move(shown, card.place);
                if moves {
                    look.actions = card
                        .place
                        .moves
                        .iter()
                        .map(|to| {
                            Action::new(
                                to.phrase().text(lang),
                                Weight::Secondary,
                                UpdateButton::Move(*to),
                            )
                        })
                        .collect();
                    look.lines
                        .push(Phrase::UpdateMoveWhat.text(lang).to_owned());
                    look.lines.extend(card.place.failed.clone());
                }
                let weight = if moves {
                    Weight::Ghost
                } else {
                    Weight::Secondary
                };
                look.actions.push(Action::notes(lang, page, weight));
                look
            }
            Shown::Updated { version, page } => {
                let mut look = Self::bare(DONE, Phrase::UpdatedTo.fill(lang, &[version]));
                if let Some(page) = page {
                    look.actions.push(Action::notes(lang, page, Weight::Ghost));
                }
                look
            }
        }
    }
}
