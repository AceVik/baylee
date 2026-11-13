//! Valuing new tokens and a private selection that grants a cast permission.
use super::{
    Aim, CARD, CounterKind, Filter, HeuristicAgent, KeywordSet, ObjectStatus, Origin, PlayerRel,
    PlayerView, PublicObject, TypeSet, ZoneRef, creature,
};
use baylee_cards_dsl::CopyMod;
use baylee_core::types::SubtypeSet;

/// The keyword is useful once, irrespective of how many identical counters
/// are placed. There is no recipient before this resolution creates a token.
pub(super) fn counter_bonus(kind: CounterKind, amount: i64, created: i64) -> Option<i64> {
    if amount <= 0 || created == 0 {
        return Some(0);
    }
    let per_token = match kind {
        CounterKind::Plus { power, toughness } => {
            amount * (i64::from(power) * 110 + i64::from(toughness) * 30)
        }
        _ if kind.keyword().is_some() => 60,
        _ => return None,
    };
    Some(created.saturating_mul(per_token))
}

/// Like `DrawRevealDiscardUnless`, an unknown card is estimated at even odds
/// of matching. Looking at several improves selection, never grants several
/// cards or draws from an empty library. Only the public library size is read.
pub(super) fn exile_selection(view: &PlayerView, count: u8, cast_types: TypeSet) -> i64 {
    if cast_types.is_empty() {
        return 0;
    }
    let seen = u32::from(count).min(view.seat(view.seat).map_or(0, |seat| seat.library_count));
    let miss = (0..seen).fold(CARD, |value, _| value / 2);
    CARD - miss
}

/// Count only the selected public graveyards and value the copies with their
/// own exceptions. In particular, Grist makes 1/1s, not the printed bodies.
pub(super) fn graveyard_copies(
    agent: &HeuristicAgent,
    view: &PlayerView,
    origin: Origin<'_>,
    aim: Aim,
    player: PlayerRel,
    filter: &Filter,
    mods: &[CopyMod],
) -> Option<(i64, i64)> {
    let mut worth = 0_i64;
    let mut made = 0_i64;
    for player in agent.players(view, player, aim)? {
        for card in view.graveyards.get(usize::from(player.get()))? {
            if !agent.filter_matches(filter, view, card, ZoneRef::Graveyard, Some(origin.id))? {
                continue;
            }
            let mut copy = card.clone();
            copy.controller = view.seat;
            copy.owner = view.seat;
            copy.commander = false;
            copy.counters.clear();
            copy.status = ObjectStatus::NONE;
            for modifier in mods {
                apply(&mut copy, modifier)?;
            }
            if !creature(&copy) || copy.toughness.is_some_and(|t| t > 0) {
                worth = worth.saturating_add(
                    HeuristicAgent::permanent_worth(view, &copy) + agent.etb_worth(view, &copy),
                );
            }
            made += 1;
        }
    }
    Some((worth, made))
}

fn apply(copy: &mut PublicObject, modifier: &CopyMod) -> Option<()> {
    match *modifier {
        CopyMod::SetPT(power, toughness) => {
            copy.power = Some(power);
            copy.toughness = Some(toughness);
        }
        CopyMod::SetColor(colors) => copy.colors = colors,
        CopyMod::SetCreatureTypes(types) => {
            copy.keywords &= !KeywordSet::CHANGELING.bits();
            copy.subtypes = copy
                .subtypes
                .difference(SubtypeSet::ALL_CREATURE)
                .union(SubtypeSet::from_slice(types));
        }
        CopyMod::AddKeyword(keywords) => copy.keywords |= keywords.bits(),
        CopyMod::AddType(types) => copy.types = copy.types.union(types),
        CopyMod::RemoveType(types) => copy.types = copy.types.difference(types),
        CopyMod::AddSupertype(types) => copy.supertypes = copy.supertypes.union(types),
        CopyMod::RemoveSupertype(types) => copy.supertypes = copy.supertypes.difference(types),
        CopyMod::AddSubtype(subtype) => copy.subtypes.insert(subtype),
        CopyMod::NoManaCost => copy.mana_value = 0,
        _ => return None,
    }
    Some(())
}
