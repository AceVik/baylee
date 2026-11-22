//! Private trophies use the current account worker. Owning the public handle alone
//! never grants access to an account's Tower or Daily/Weekly records.
use crate::insignia::Motif;
use baylee_client_core::i18n::Phrase;
use baylee_client_core::{
    challenges::api as challenge, echoes::awards_api as echoes, tower::api as tower,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Medal {
    pub kind: Motif,
    pub tier: challenge::Tier,
    pub count: u64,
    pub next: Option<u64>,
}

impl Medal {
    pub fn title(self) -> Phrase {
        match self.kind {
            Motif::Challenge(kind) => match kind {
                challenge::BadgeKind::Daily => Phrase::ChallengeBadgeDaily,
                challenge::BadgeKind::Weekly => Phrase::ChallengeBadgeWeekly,
                challenge::BadgeKind::Tactician => Phrase::ChallengeBadgeTactician,
                challenge::BadgeKind::FirstTry => Phrase::ChallengeBadgeFirst,
                challenge::BadgeKind::Regular => Phrase::ChallengeBadgeRegular,
            },
            Motif::Tower(kind) => match kind {
                tower::BadgeKind::Pathfinder => Phrase::TowerPathfinder,
                tower::BadgeKind::Summit => Phrase::TowerSummit,
                tower::BadgeKind::Unbroken => Phrase::TowerUnbroken,
            },
            Motif::Echoes(kind) => match kind {
                echoes::BadgeKind::Victories => Phrase::EchoesBadgeVictories,
                echoes::BadgeKind::Ascents => Phrase::EchoesBadgeAscents,
                echoes::BadgeKind::Unbroken => Phrase::EchoesBadgeUnbroken,
            },
        }
    }

    pub fn material(self) -> Phrase {
        match self.tier {
            challenge::Tier::Bronze => Phrase::ChallengeBronze,
            challenge::Tier::Silver => Phrase::ChallengeSilver,
            challenge::Tier::Gold => Phrase::ChallengeGold,
            challenge::Tier::Platinum => Phrase::ChallengePlatinum,
            challenge::Tier::Obsidian => Phrase::ChallengeObsidian,
        }
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct Collection {
    pub groups: [Option<Vec<Medal>>; 3],
    pub failed: bool,
}

impl Collection {
    pub fn read(account: &baylee_account::Account) -> Self {
        let challenge = match account.challenges(challenge::Request::Progress) {
            Ok(challenge::Response::Progress { badges, .. })
                if badges.len() == challenge::BadgeKind::ALL.len() =>
            {
                Some(
                    badges
                        .into_iter()
                        .filter_map(|b| {
                            let b = b.kind.progress(b.count);
                            Some(Medal {
                                kind: Motif::Challenge(b.kind),
                                tier: b.tier?,
                                count: u64::from(b.count),
                                next: b.next.map(u64::from),
                            })
                        })
                        .collect(),
                )
            }
            _ => None,
        };
        let tower = match account.challenges(challenge::Request::Tower {
            request: tower::Request::Read {
                season: None,
                board: tower::Board::Ascent,
                offset: 0,
            },
        }) {
            Ok(challenge::Response::Tower { response }) => match *response {
                tower::Response::Read { badges, .. } => Some(
                    badges
                        .into_iter()
                        .take(3)
                        .filter_map(|b| {
                            Some(Medal {
                                kind: Motif::Tower(b.kind),
                                tier: b.tier?,
                                count: u64::from(b.count),
                                next: b.next.map(u64::from),
                            })
                        })
                        .collect(),
                ),
                _ => None,
            },
            _ => None,
        };
        let echoes = account
            .tower_award_progress()
            .ok()
            .filter(|p| p.validate().is_ok())
            .map(|p| {
                p.badges()
                    .into_iter()
                    .filter_map(|b| {
                        Some(Medal {
                            kind: Motif::Echoes(b.kind),
                            tier: b.tier?,
                            count: b.count,
                            next: b.next,
                        })
                    })
                    .collect()
            });
        let groups = [challenge, tower, echoes];
        let failed = groups.iter().any(Option::is_none);
        Self { groups, failed }
    }

    pub fn merge(&mut self, latest: Self) {
        self.failed = latest.failed;
        for (confirmed, read) in self.groups.iter_mut().zip(latest.groups) {
            if read.is_some() {
                *confirmed = read;
            }
        }
    }
}
