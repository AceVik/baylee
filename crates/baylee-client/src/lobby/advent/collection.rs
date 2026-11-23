//! Shared provenance for the private collection, earned reward and deck picker.
use crate::lobby::ranked::{divisions, rank_name};
use baylee_account::{
    advent::collection::{Item, Source},
    ranked::{League, Rank},
};
use baylee_client_core::{Lang, Phrase};
#[cfg(test)]
pub(in crate::lobby) mod tests;

pub(in crate::lobby) fn title(item: &Item, lang: Lang) -> &str {
    if lang == Lang::De {
        &item.title.de
    } else {
        &item.title.en
    }
}

pub(super) fn source_label(source: Source, lang: Lang) -> Option<String> {
    match source {
        Source::Advent { year, door } if year > 0 && (1..=24).contains(&door) => Some(format!(
            "{year} · {}",
            Phrase::AdventDoor.fill(lang, &[&door.to_string()])
        )),
        _ => ranked_label(source, lang),
    }
}

pub(in crate::lobby) fn ranked_label(source: Source, lang: Lang) -> Option<String> {
    let Source::Ranked { league, division } = source else {
        return None;
    };
    let count = divisions(league);
    let valid = division.map_or(count == 0, |n| (1..=count).contains(&n));
    if league == League::Unranked || !valid {
        return None;
    }
    let rank = rank_name(
        Rank {
            league,
            division,
            champion_place: None,
        },
        lang,
    );
    Some(Phrase::CosmeticRankReward.fill(lang, &[&rank]))
}
