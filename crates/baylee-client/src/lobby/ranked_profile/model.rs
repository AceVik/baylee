//! Match public profiles to the requested full handle or published account ID.
use crate::rank_art::RankArt;
use baylee_account::ranked::{
    Award, Id, League, ProfileTarget, Rank, Response, Season, Standing, Track,
};
use baylee_client_core::i18n::Lang;

/// UUID keys and full handles occupy disjoint parts of the profile cache.
pub(super) fn account_key(id: Id) -> String {
    format!("@{}", uuid::Uuid::from_bytes(id))
}

pub(super) fn account_id(key: &str) -> Option<Id> {
    Some(
        uuid::Uuid::parse_str(key.strip_prefix('@')?)
            .ok()?
            .into_bytes(),
    )
}

pub(super) fn target(key: &str) -> Option<ProfileTarget> {
    account_id(key)
        .map(ProfileTarget::Account)
        .or_else(|| valid_handle(key).then(|| ProfileTarget::Handle(key.to_owned())))
}

pub(crate) fn valid_handle(handle: &str) -> bool {
    let Some((name, tag)) = handle.rsplit_once('#') else {
        return false;
    };
    (4..=8).contains(&tag.len())
        && baylee_account::profile::valid_display_name(name)
        && tag.bytes().all(|b| b.is_ascii_hexdigit())
        && u32::from_str_radix(tag, 16).is_ok_and(|n| {
            baylee_account::profile::valid_tag(n) && (tag.len() == 4 || !tag.starts_with('0'))
        })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Profile {
    pub account: Id,
    pub season: Season,
    pub formats: Vec<Standing>,
    pub awards: Vec<Award>,
}

impl Profile {
    pub fn accept(wanted: &str, response: Response) -> Option<Self> {
        let Response::Profile {
            season,
            account,
            name,
            tag,
            formats,
            awards,
            ..
        } = response
        else {
            return None;
        };
        // The profile projects one player and one best deck per format.
        // 256 also covers every distinct league/division, podium, track and format.
        let matches = match account_id(wanted) {
            Some(id) => id == account,
            None => wanted.rsplit_once('#').zip(name.zip(tag)).is_some_and(
                |((wanted_name, wanted_tag), (name, tag))| {
                    wanted_name == name
                        && wanted_tag.eq_ignore_ascii_case(&baylee_account::profile::tag_text(tag))
                },
            ),
        };
        if !matches || formats.len() > 6 || awards.len() > 256 || season.name.len() > 256 {
            return None;
        }
        Some(Self {
            account,
            season,
            formats,
            awards,
        })
    }

    pub fn best(&self) -> Option<Rank> {
        self.formats
            .iter()
            .filter(|s| s.track == Track::Player && s.career.games > 0)
            .max_by_key(|s| {
                (
                    s.rank.league,
                    std::cmp::Reverse(s.rank.division),
                    std::cmp::Reverse(s.rank.champion_place),
                    s.elo,
                )
            })
            .map(|s| s.rank)
    }
}

pub(super) use crate::lobby::ranked::{format_name, rank_name};

pub(super) fn badge(rank: Rank, track: Track) -> Option<RankArt> {
    RankArt::badge(
        crate::lobby::ranked::league_name(rank.league, Lang::En),
        rank.division,
        track == Track::Deck,
        rank.champion_place.map(u32::from),
    )
}

pub(super) fn frame(rank: Rank) -> Option<RankArt> {
    RankArt::frame(
        crate::lobby::ranked::league_name(rank.league, Lang::En),
        rank.division,
        rank.champion_place.map(u32::from),
    )
}

pub(super) fn deck_frame(rank: Rank) -> Option<RankArt> {
    RankArt::deck_frame(
        crate::lobby::ranked::league_name(rank.league, Lang::En),
        rank.division,
        rank.champion_place.map(u32::from),
    )
}

/// The next display title; promotion eligibility itself always comes from Central.
pub(super) fn next(rank: Rank) -> Option<Rank> {
    if let Some(division) = rank.division.filter(|v| *v > 1) {
        return Some(Rank {
            division: Some(division - 1),
            ..rank
        });
    }
    let (league, division) = match rank.league {
        League::Unranked => (League::Paper, None),
        League::Paper => (League::Wood, Some(3)),
        League::Wood => (League::Iron, Some(3)),
        League::Iron => (League::Bronze, Some(3)),
        League::Bronze => (League::Silver, Some(3)),
        League::Silver => (League::Gold, Some(4)),
        League::Gold => (League::Platinum, Some(4)),
        League::Platinum => (League::Diamond, Some(4)),
        League::Diamond => (League::Master, None),
        League::Master => (League::Grandmaster, None),
        League::Grandmaster => (League::Champion, None),
        League::Champion => {
            return rank.champion_place.filter(|v| *v > 1).map(|p| Rank {
                champion_place: Some(p - 1),
                ..rank
            });
        }
    };
    Some(Rank {
        league,
        division,
        champion_place: None,
    })
}
