//! Public profile data is accepted only for the exact handle Central returned.
use crate::rank_art::RankArt;
use baylee_account::ranked::{Award, Format, Id, League, Rank, Response, Season, Standing, Track};
use baylee_client_core::i18n::Lang;

pub(super) const FORMATS: [Format; 3] = [Format::Standard, Format::Highlander, Format::Commander];

pub(super) fn valid_handle(handle: &str) -> bool {
    let Some((name, tag)) = handle.rsplit_once('#') else {
        return false;
    };
    !name.is_empty()
        && name.len() <= 160
        && !name.chars().any(char::is_control)
        && (4..=8).contains(&tag.len())
        && tag.bytes().all(|b| b.is_ascii_hexdigit())
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
            name: Some(name),
            tag: Some(tag),
            formats,
            awards,
            ..
        } = response
        else {
            return None;
        };
        // The profile projects one player and one best deck per format.
        // 256 also covers every distinct league/division, podium, track and format.
        if wanted != format!("{name}#{tag:04x}")
            || formats.len() > 6
            || awards.len() > 256
            || season.name.len() > 256
        {
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
