//! Deterministic answers to mandatory mana and typed word choices.

use baylee_cards_dsl::{KeywordSet, TextWordKind};
use baylee_core::color::Color;
use baylee_core::ids::DamageSourceRef;
use baylee_engine::choice::{ManaAbilityChoice, ManaChoiceId, PlayerAction};
use baylee_view::PlayerView;

pub(crate) fn mana_choice(
    choice: ManaChoiceId,
    options: &[ManaAbilityChoice],
) -> Option<PlayerAction> {
    options
        .first()
        .map(|option| PlayerAction::ChooseManaAbility {
            choice,
            source: option.source,
            ability_index: option.ability_index,
        })
}

/// Prefer a word actually present in a public keyword or basic land type.
/// Unrecognized rules still receive a legal distinct pair, without inspecting
/// a later incarnation or private card identity.
pub(crate) fn text_word(view: &PlayerView, kind: TextWordKind, target: DamageSourceRef) -> u32 {
    let Some(object) = view.target_object(target) else {
        return 0;
    };
    let keywords = match kind {
        TextWordKind::Color => [
            KeywordSet::EMPTY,
            KeywordSet::EMPTY,
            KeywordSet::PROTECTION_BLACK,
            KeywordSet::EMPTY,
            KeywordSet::EMPTY,
        ],
        TextWordKind::BasicLandType => [
            KeywordSet::PLAINSWALK,
            KeywordSet::ISLANDWALK,
            KeywordSet::SWAMPWALK,
            KeywordSet::MOUNTAINWALK,
            KeywordSet::FORESTWALK,
        ],
    };
    let from = keywords
        .iter()
        .position(|keyword| object.keywords & keyword.bits() != 0)
        .or_else(|| {
            (kind == TextWordKind::BasicLandType && object.is_current)
                .then(|| view.object(target.object))
                .flatten()
                .and_then(|current| {
                    baylee_engine::text_changes::BASIC_LAND_TYPES
                        .iter()
                        .position(|&land| current.subtypes.contains(land))
                })
        })
        .unwrap_or(0);
    let to = (0..5)
        .filter(|&index| index != from)
        .max_by_key(|&index| {
            let relevant = view
                .battlefield
                .iter()
                .filter(|other| other.controller != object.controller)
                .filter(|other| match kind {
                    TextWordKind::Color => other.colors.contains(Color::ALL[index]),
                    TextWordKind::BasicLandType => other
                        .subtypes
                        .contains(baylee_engine::text_changes::BASIC_LAND_TYPES[index]),
                })
                .count();
            (relevant, std::cmp::Reverse(index))
        })
        .unwrap_or((from + 1) % 5);
    u32::try_from(from * 4 + to - usize::from(to > from)).unwrap_or(0)
}
