//! The coarse classes of an answer at priority (rule `coarse_class_v1`): one
//! definition for the converter that writes them beside a distillation
//! dataset and for the player that decodes by them
//! (`netplay4::grouped`), so the two cannot drift apart.

use baylee_cards_dsl::AbilityDef;
use baylee_engine::choice::PlayerAction;
use baylee_view::{PlayerView, PublicObject, RulesFace};

use crate::features4::EncodedCandidates4;

/// The coarse classes by code: pass (`PassPriority`, `CancelCast`), tap
/// (every mana ability, whether offered as `ActivateManaAbility` or through
/// `ActivateAbility`: a dual land's colour, an Eldrazi Scion), cast, activate
/// (a non-mana ability), land, other.
pub const COARSE_CLASSES: [&str; 6] = ["pass", "tap", "cast", "activate", "land", "other"];

/// The coarse class rule's name, for the files that carry the classes.
pub const COARSE_CLASS_RULE: &str = "coarse_class_v1";

/// The code of `pass` in [`COARSE_CLASSES`].
pub const PASS_CLASS: u8 = 0;

/// A coarse class code for no candidate.
pub const NO_CLASS: u8 = 255;

/// The abilities of a permanent or token, as the view shows it.
fn abilities_of(object: &PublicObject) -> Option<&'static [AbilityDef]> {
    if let Some(face) = object.rules.or_else(|| object.card.map(RulesFace::from)) {
        return baylee_cards::by_index(face.card)
            .map(|def| def.abilities_for_face(usize::from(face.face)));
    }
    object
        .token
        .and_then(baylee_cards::tokens::by_token_id)
        .map(|token| token.abilities)
}

/// Whether an action is a mana ability: `ActivateManaAbility` or
/// `ChooseManaAbility`, or an `ActivateAbility` whose ability is one.
#[must_use]
pub fn is_mana_ability_action(view: &PlayerView, action: &PlayerAction) -> bool {
    match action {
        PlayerAction::ActivateManaAbility { .. } | PlayerAction::ChooseManaAbility { .. } => true,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } => view
            .object(*source)
            .and_then(abilities_of)
            .and_then(|abilities| abilities.get(*ability_index as usize))
            .is_some_and(AbilityDef::is_mana_ability),
        _ => false,
    }
}

/// The coarse class code of an answer at priority ([`COARSE_CLASSES`]);
/// `tap` says whether it is a mana ability ([`is_mana_ability_action`]).
#[must_use]
pub const fn coarse_class(action: Option<&PlayerAction>, tap: bool) -> u8 {
    match action {
        Some(PlayerAction::PassPriority | PlayerAction::CancelCast) => PASS_CLASS,
        _ if tap => 1,
        Some(
            PlayerAction::CastSpell { .. }
            | PlayerAction::Suspend { .. }
            | PlayerAction::Foretell { .. },
        ) => 2,
        Some(PlayerAction::ActivateAbility { .. }) => 3,
        Some(PlayerAction::PlayLand { .. }) => 4,
        _ => 5,
    }
}

/// The coarse classes of a priority question's candidates, in menu order.
#[must_use]
pub fn coarse_classes_of(cands: &EncodedCandidates4, view: &PlayerView) -> Vec<u8> {
    cands
        .candidates
        .iter()
        .map(|c| {
            let tap = c
                .action
                .as_ref()
                .is_some_and(|a| is_mana_ability_action(view, a));
            coarse_class(c.action.as_ref(), tap)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::ObjectId;

    #[test]
    fn a_coarse_class_is_what_an_answer_does() {
        let card = ObjectId::new(7, 0);
        let ability = PlayerAction::ActivateAbility {
            source: card,
            ability_index: 0,
        };
        for (action, tap, class) in [
            (Some(PlayerAction::PassPriority), false, "pass"),
            (Some(PlayerAction::CancelCast), false, "pass"),
            (
                Some(PlayerAction::ActivateManaAbility { source: card }),
                true,
                "tap",
            ),
            // A mana ability offered through ActivateAbility is a tap.
            (Some(ability.clone()), true, "tap"),
            (Some(ability), false, "activate"),
            (Some(PlayerAction::CastSpell { card }), false, "cast"),
            (Some(PlayerAction::PlayLand { card }), false, "land"),
            (Some(PlayerAction::MulliganKeep), false, "other"),
            (None, false, "other"),
        ] {
            let code = coarse_class(action.as_ref(), tap);
            assert_eq!(COARSE_CLASSES[usize::from(code)], class, "{action:?}");
        }
        assert_eq!(COARSE_CLASSES[usize::from(PASS_CLASS)], "pass");
        assert_eq!(COARSE_CLASS_RULE, "coarse_class_v1");
    }
}
