//! What a permanent can do, and what to call it.
//!
//! The client could not activate an ability at all before this: clicking a
//! permanent selected it for whatever choice was pending, and a Forest, a
//! mana dork and a planeswalker were all equally inert. `Interaction::activate`
//! existed and nothing called it.
//!
//! What is here is the list, in a stable order, built only from what the
//! engine offered — and a label for each, which is the part that needs the
//! card registry and is therefore the reason this is not in
//! `baylee-client-core`. "Ability 2" is a label a player has to guess at;
//! "Tap for {G}" and "+1" are not.

use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::interaction::Interaction;
use baylee_client_core::manaplan::Tap;
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaColor;
use baylee_engine::choice::PlayerAction;
use baylee_view::PlayerView;

use baylee_cards_dsl::{AbilityDef, Cost, CostPart};
use baylee_core::mana::ManaCost;

/// One thing a permanent is offering to do.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AbilityOption {
    /// The action that does it — built through [`Interaction::activate`], so
    /// it is one the engine listed.
    pub action: PlayerAction,
    /// What the button says.
    pub label: String,
    /// Whether this is a mana ability (CR 605.1).
    ///
    /// The one exception to arm-then-act: floating mana is the cheap mistake,
    /// so a mana ability stays one tap. Not `legal.mana_abilities`, which
    /// carries the CR 305.6 shortcut and granted abilities but not a printed
    /// `{T}: Add {G}` — a mana dork would otherwise ask for a confirmation a
    /// basic land does not.
    pub mana: bool,
}

/// Everything `object` is offering right now, in a stable order.
///
/// Stable because the prompt bar draws it as a row of buttons and a list that
/// reordered under a player would activate the wrong thing: the CR 305.6
/// shortcut first, then printed abilities by index.
#[must_use]
pub fn options(
    lang: Lang,
    view: &PlayerView,
    interaction: &Interaction,
    object: ObjectId,
) -> Vec<AbilityOption> {
    let Some(legal) = interaction.legal_actions() else {
        return Vec::new();
    };
    let mut out = Vec::new();

    // The mana half comes through `manasources`, which has already reduced a
    // permanent to the one tap it actually has. A Forest offers its CR 305.6
    // shortcut *and* the `{T}: Add {G}` printed on the card, and a chooser
    // that listed both would be offering the same button twice.
    if let Some(source) = crate::manasources::sources(view, legal)
        .into_iter()
        .find(|s| s.id == object)
    {
        // Built from the tap rather than through `Interaction::activate`,
        // which reads index 0 as the CR 305.6 shortcut whenever the object
        // has one. A permanent with both — a typed land whose printed ability
        // at index 0 makes *two* mana — would otherwise be sent the shortcut
        // and make one.
        let action = match source.tap {
            Tap::Intrinsic => interaction.activate(object, 0),
            Tap::Ability(index) => legal.abilities.contains(&(object, index)).then_some(
                PlayerAction::ActivateAbility {
                    source: object,
                    ability_index: index,
                },
            ),
        };
        if let Some(action) = action {
            out.push(AbilityOption {
                action,
                label: mana_label(lang, &source),
                mana: true,
            });
        }
    }

    for &(source, index) in &legal.abilities {
        if source != object {
            continue;
        }
        // Already offered above, whichever of them won.
        if crate::manasources::printed_source(view, object, index).is_some() {
            continue;
        }
        let Some(action) = interaction.activate(object, index) else {
            continue;
        };
        // The two synthetic indices are not positions on the card, so neither
        // the registry nor the card's ability list has anything to say about
        // them — and the fallback label counts them out as "Ability N", which
        // on `GRANTED_ABILITY` overflows: in a debug build the client dies the
        // moment a Chromatic Lantern's land is under the pointer, and in a
        // release build the button reads "Ability 0". For the granted one the
        // engine has already answered the question that decides the button —
        // whether tapping it is a mana ability — and a prepared cast is a
        // cast, never a mana ability.
        let (label, mana) = match index {
            baylee_engine::choice::GRANTED_ABILITY => (
                Phrase::GrantedAbility.text(lang).to_string(),
                legal.mana_abilities.contains(&object),
            ),
            baylee_engine::choice::PREPARED_CAST => {
                (Phrase::PreparedCast.text(lang).to_string(), false)
            }
            _ => (
                printed_label(lang, view, object, index),
                makes_mana(view, object, index),
            ),
        };
        out.push(AbilityOption {
            action,
            label,
            mana,
        });
    }
    out
}

/// Whether a printed ability is a mana ability (CR 605.1).
///
/// Read off the card's own `mana_ability` flag, which is the only answer that
/// is true for every card. `manasources` reduces a permanent to the *one* tap
/// it usually has, which is right for a mana plan and wrong here: Yavimaya
/// Coast prints two mana abilities, and the second would have asked for a
/// confirmation the first does not.
fn makes_mana(view: &PlayerView, object: ObjectId, index: u32) -> bool {
    matches!(
        crate::manasources::ability_at(view, object, index),
        Some(
            AbilityDef::Activated {
                mana_ability: true,
                ..
            } | AbilityDef::ActivatedConditional {
                mana_ability: true,
                ..
            }
        )
    )
}

/// "Tap for {G}", or "Tap for WUBRG" where there is a choice to make.
fn mana_label(lang: Lang, source: &baylee_client_core::manaplan::Source) -> String {
    let colors: String = source.colors.iter().map(|c| pip(*c)).collect();
    if source.amount > 1 && source.colors.len() == 1 {
        return Phrase::TapFor.fill(lang, &[&colors.repeat(source.amount as usize)]);
    }
    Phrase::TapFor.fill(lang, &[&colors])
}

/// A printed ability's label: a planeswalker's loyalty cost, otherwise what
/// the ability costs to activate.
///
/// Deliberately short — this is a button on a bar that already carries the
/// prompt, and a player who needs the full wording has the card's own text a
/// hover away. But short is not the same as opaque: "Ability 2" is a label a
/// player has to count out on the card, and it was the only one this could
/// produce. `{2}, {T}` is read at a glance and is the half of an ability a
/// player is actually deciding about.
fn printed_label(lang: Lang, view: &PlayerView, object: ObjectId, index: u32) -> String {
    let unnamed = || Phrase::AbilityNumbered.fill(lang, &[&(index + 1).to_string()]);
    let Some(def) = crate::manasources::ability_at(view, object, index) else {
        return unnamed();
    };
    match def {
        AbilityDef::Loyalty { cost, .. } => {
            if *cost >= 0 {
                format!("+{cost}")
            } else {
                format!("\u{2212}{}", -cost)
            }
        }
        AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. } => {
            cost_label(lang, cost).unwrap_or_else(unnamed)
        }
        _ => unnamed(),
    }
}

/// What an activated ability costs, as one short string.
///
/// `None` for a free ability: "" is not a button and "Free" would be a claim
/// about the *effect* rather than the cost, so the caller falls back to the
/// ability's position instead.
fn cost_label(lang: Lang, cost: &Cost) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if cost.mana != ManaCost::ZERO {
        parts.push(cost.mana.to_string());
    }
    for part in cost.parts {
        parts.push(match part {
            CostPart::TapSelf => "{T}".to_string(),
            CostPart::UntapSelf => "{Q}".to_string(),
            CostPart::SacrificeSelf => Phrase::CostSacrificeThis.text(lang).to_string(),
            CostPart::Sacrifice(_) => Phrase::CostSacrifice.text(lang).to_string(),
            CostPart::PayLife(n) => Phrase::CostPayLife.fill(lang, &[&n.to_string()]),
            CostPart::PayLifeX => Phrase::CostPayXLife.text(lang).to_string(),
            CostPart::Discard(_) => Phrase::CostDiscard.text(lang).to_string(),
            CostPart::DiscardSelf => Phrase::CostDiscardThis.text(lang).to_string(),
            CostPart::ExileSelf => Phrase::CostExileThis.text(lang).to_string(),
            CostPart::ReturnSelfToHand => Phrase::CostReturnThis.text(lang).to_string(),
            CostPart::ExileFromHand(_) => Phrase::CostExileACard.text(lang).to_string(),
        });
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// One mana symbol, as a letter.
const fn pip(color: ManaColor) -> char {
    match color {
        ManaColor::White => 'W',
        ManaColor::Blue => 'U',
        ManaColor::Black => 'B',
        ManaColor::Red => 'R',
        ManaColor::Green => 'G',
        ManaColor::Colorless => 'C',
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::ids::PlayerId;
    use baylee_engine::choice::{GRANTED_ABILITY, LegalActions, PREPARED_CAST, Pending};

    fn offering(abilities: Vec<(ObjectId, u32)>, mana: Vec<ObjectId>) -> Interaction {
        Interaction::new(
            Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(LegalActions {
                    abilities,
                    mana_abilities: mana,
                    ..LegalActions::default()
                }),
            },
            PlayerId::new(0),
        )
    }

    /// The two synthetic indices are offered like any other ability and have
    /// to be labelled without asking the card about them.
    ///
    /// `GRANTED_ABILITY` is `u32::MAX`, and the fallback label counts an
    /// ability out as `index + 1` — so a permanent under a Chromatic Lantern
    /// killed the client outright in a debug build, on the frame the chooser
    /// was built. Nothing in the duel-flow ledger reaches this: it drives
    /// `Interaction`, not the list of buttons drawn from it.
    #[test]
    fn a_granted_ability_is_labelled_without_asking_the_card_it_is_not_on() {
        let id = ObjectId::new(1, 0);
        let view = ViewBuilder::new(2)
            .with_battlefield(0, [token(1, 0, "Ally", 2, 2)])
            .build();

        // Granted, and the engine says it makes mana — so it is one tap, not
        // arm-then-act.
        let i = offering(vec![(id, GRANTED_ABILITY)], vec![id]);
        let out = options(Lang::En, &view, &i, id);
        assert_eq!(out.len(), 1, "one granted ability, one button: {out:?}");
        assert_eq!(out[0].label, "Granted ability");
        assert!(out[0].mana, "the engine offered it as a mana ability");
        assert_eq!(
            out[0].action,
            PlayerAction::ActivateAbility {
                source: id,
                ability_index: GRANTED_ABILITY,
            }
        );

        // The same ability granted by something that is not a mana source.
        let i = offering(vec![(id, GRANTED_ABILITY)], vec![]);
        assert!(
            !options(Lang::En, &view, &i, id)[0].mana,
            "nothing else can tell the client this, so it must be the offer"
        );

        // A prepared cast is a cast: never a mana ability, and never
        // "Ability 4294967295".
        let i = offering(vec![(id, PREPARED_CAST)], vec![]);
        let out = options(Lang::En, &view, &i, id);
        assert_eq!(out[0].label, "Cast the prepared spell");
        assert!(!out[0].mana);
    }
}
