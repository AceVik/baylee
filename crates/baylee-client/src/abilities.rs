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
}

/// Everything `object` is offering right now, in a stable order.
///
/// Stable because the prompt bar draws it as a row of buttons and a list that
/// reordered under a player would activate the wrong thing: the CR 305.6
/// shortcut first, then printed abilities by index.
#[must_use]
pub fn options(
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
                label: mana_label(&source),
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
        out.push(AbilityOption {
            action,
            label: printed_label(view, object, index),
        });
    }
    out
}

/// "Tap for {G}", or "Tap for WUBRG" where there is a choice to make.
fn mana_label(source: &baylee_client_core::manaplan::Source) -> String {
    let colors: String = source.colors.iter().map(|c| pip(*c)).collect();
    if source.amount > 1 && source.colors.len() == 1 {
        return format!("Tap for {}", colors.repeat(source.amount as usize));
    }
    format!("Tap for {colors}")
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
fn printed_label(view: &PlayerView, object: ObjectId, index: u32) -> String {
    let unnamed = || format!("Ability {}", index + 1);
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
            cost_label(cost).unwrap_or_else(unnamed)
        }
        _ => unnamed(),
    }
}

/// What an activated ability costs, as one short string.
///
/// `None` for a free ability: "" is not a button and "Free" would be a claim
/// about the *effect* rather than the cost, so the caller falls back to the
/// ability's position instead.
fn cost_label(cost: &Cost) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if cost.mana != ManaCost::ZERO {
        parts.push(cost.mana.to_string());
    }
    for part in cost.parts {
        parts.push(match part {
            CostPart::TapSelf => "{T}".to_string(),
            CostPart::UntapSelf => "{Q}".to_string(),
            CostPart::SacrificeSelf => "Sacrifice this".to_string(),
            CostPart::Sacrifice(_) => "Sacrifice".to_string(),
            CostPart::PayLife(n) => format!("Pay {n} life"),
            CostPart::PayLifeX => "Pay X life".to_string(),
            CostPart::Discard(_) => "Discard".to_string(),
            CostPart::DiscardSelf => "Discard this".to_string(),
            CostPart::ExileSelf => "Exile this".to_string(),
            CostPart::ReturnSelfToHand => "Return this".to_string(),
            CostPart::ExileFromHand(_) => "Exile a card".to_string(),
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
