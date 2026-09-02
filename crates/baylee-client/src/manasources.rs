//! What the seat can tap for mana, read off the choice the engine offered.
//!
//! [`baylee_client_core::manaplan`] decides *which* sources to tap; this is
//! the half that cannot live there, because knowing that ability 2 of a
//! Command Tower makes mana takes the compiled card registry, and
//! `baylee-client-core` deliberately does not link it.
//!
//! The list is built from `LegalActions` and nothing else, so a source that
//! is tapped, summoning sick, or otherwise unavailable never appears — the
//! engine already answered that question and this does not second-guess it.
//! What the registry adds is only *what comes out*.

use baylee_client_core::manaplan::{Source, Tap, basic_land_color};
use baylee_core::mana::ManaCost;
use baylee_engine::choice::LegalActions;
use baylee_view::PlayerView;

use baylee_cards_dsl::{AbilityDef, Amount, Effect, ManaSource};

/// Every mana source the seat may tap right now.
///
/// **One entry per permanent**, which is the whole subtlety here. A Forest
/// appears twice in `LegalActions` — once in `mana_abilities` as the CR 305.6
/// shortcut and once in `abilities` as the `{T}: Add {G}` printed on the card
/// — and it can still only be tapped once. Two entries would let the planner
/// pay `{G}{G}` with one Forest, which is a plan the engine refuses after the
/// land is already tapped.
#[must_use]
pub fn sources(view: &PlayerView, legal: &LegalActions) -> Vec<Source> {
    let mut sources = Vec::new();

    // The CR 305.6 shortcut. The engine offers it only for a land with
    // exactly one basic type, and the colour follows from the *projected*
    // subtypes — an animated Dryad Arbor still taps for green.
    for &id in &legal.mana_abilities {
        let Some(object) = view.battlefield.iter().find(|o| o.id == id) else {
            continue;
        };
        if let Some(color) = basic_land_color(&object.subtypes) {
            sources.push(Source::fixed(id, Tap::Intrinsic, color));
        }
    }

    // Printed mana abilities: Command Tower, a Llanowar Elf, a Sol Ring.
    for &(id, index) in &legal.abilities {
        if let Some(source) = printed_source(view, id, index) {
            sources.push(source);
        }
    }

    // A permanent taps once, so it is one source: the best of whatever it
    // offered. "Best" is the one that leaves the planner the most room — more
    // mana first, then more colours — and the intrinsic shortcut wins a tie
    // because it costs one fewer round trip and is never asked for a colour.
    sources.sort_by(|a, b| {
        a.id.cmp(&b.id)
            .then_with(|| b.amount.cmp(&a.amount))
            .then_with(|| b.colors.len().cmp(&a.colors.len()))
            .then_with(|| matches!(a.tap, Tap::Ability(_)).cmp(&matches!(b.tap, Tap::Ability(_))))
    });
    sources.dedup_by(|a, b| a.id == b.id);
    sources
}

/// Ability `index` of `object`, out of the registry.
///
/// The face matters: an MDFC's back has its own abilities, and reading the
/// front's list for it would name the wrong one.
#[must_use]
pub fn ability_at(
    view: &PlayerView,
    object: baylee_core::ids::ObjectId,
    index: u32,
) -> Option<&'static AbilityDef> {
    let card = view.object(object)?.card?;
    let def = baylee_cards::by_index(card.index)?;
    let abilities = def.abilities_for_face(card.face as usize);
    abilities.get(usize::try_from(index).ok()?)
}

/// The source that ability `index` of `object` is, when it is one this client
/// can read.
///
/// Also the answer to "is this a mana ability" for the ability chooser, which
/// has to leave them out: a Forest's printed `{T}: Add {G}` is the same tap as
/// the CR 305.6 shortcut, and offering both is offering the same button twice.
#[must_use]
pub fn printed_source(
    view: &PlayerView,
    object: baylee_core::ids::ObjectId,
    index: u32,
) -> Option<Source> {
    mana_ability(object, index, ability_at(view, object, index)?)
}

/// Reads one ability as a mana source, or decides it is not one this can use.
///
/// The bar is deliberately high. An ability that costs mana to activate could
/// be worth planning through, and is not: it would make the plan recursive,
/// and a filter land the plan mis-sequences leaves the player tapped out with
/// nothing to show. Restricted mana is refused for a different reason — what
/// a Cavern of Souls' mana may be spent on is a rules question, and answering
/// it on this side of the wire is exactly the guess this whole module avoids.
fn mana_ability(
    id: baylee_core::ids::ObjectId,
    index: u32,
    ability: &'static AbilityDef,
) -> Option<Source> {
    let AbilityDef::Activated {
        cost,
        effects,
        mana_ability: true,
        ..
    } = ability
    else {
        return None;
    };
    if cost.mana != ManaCost::ZERO {
        return None;
    }
    // One effect, and it has to be the mana: an ability that also does
    // something else is one the player should decide about themselves.
    let [
        Effect::AddMana {
            source,
            amount: Amount::Fixed(amount),
            restriction: None,
            ..
        },
    ] = effects
    else {
        return None;
    };
    let colors = match source {
        ManaSource::Fixed(color) => vec![*color],
        ManaSource::Choice(colors) => colors.to_vec(),
        // Both depend on the rest of the board — a commander's identity, or
        // what someone else's lands can make. The engine knows; this does not.
        ManaSource::CommanderIdentity | ManaSource::LandColor { .. } => return None,
    };
    Some(Source {
        id,
        tap: Tap::Ability(index),
        colors,
        amount: u8::try_from(*amount).unwrap_or(u8::MAX),
    })
}

/// The printed cost of a card in hand.
#[must_use]
pub fn hand_cost(card: &baylee_view::HandObject) -> Option<ManaCost> {
    let def = baylee_cards::by_index(card.card.index)?;
    let face = def
        .faces
        .get(card.card.face as usize)
        .or(def.faces.first())?;
    Some(face.mana_cost)
}
