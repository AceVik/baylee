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

use baylee_cards_dsl::AbilityDef;

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

    // Printed mana abilities: Command Tower, a Llanowar Elf, a Sol Ring —
    // and the one that is printed on no card, which the view carries instead.
    for &(id, index) in &legal.abilities {
        let source = if index == baylee_engine::choice::GRANTED_ABILITY {
            granted_source(view, id)
        } else {
            printed_source(view, id, index)
        };
        if let Some(source) = source {
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
/// The reading itself is [`baylee_cards_dsl::simple_mana`], which is where it
/// has to live: the same question is asked of an ability a continuous effect
/// *grants*, and that one is not printed on any card, so this module cannot
/// be the one that knows the answer.
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
    let mana = baylee_cards_dsl::simple_mana(cost, effects)?;
    Some(Source {
        id,
        tap: Tap::Ability(index),
        colors: mana.colors,
        amount: mana.amount,
    })
}

/// The mana a continuous effect grants `object` the ability to make.
///
/// Read off the view rather than the registry, because there is nothing in
/// the registry to read: a land under a Chromatic Lantern has an ability its
/// printed card does not mention. `baylee-gamehost` projects what it makes
/// (see `PublicObject::granted_mana`) precisely so this client can plan
/// through it instead of leaving the player to tap those lands by hand.
#[must_use]
pub fn granted_source(view: &PlayerView, object: baylee_core::ids::ObjectId) -> Option<Source> {
    let granted = view.object(object)?.granted_mana.as_ref()?;
    Some(Source {
        id: object,
        tap: Tap::Ability(baylee_engine::choice::GRANTED_ABILITY),
        colors: granted.colors.clone(),
        amount: granted.amount,
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

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::ids::ObjectId;
    use baylee_core::mana::ManaColor;
    use baylee_engine::choice::GRANTED_ABILITY;

    /// A land under a Chromatic Lantern, as the seat is shown it: a Mountain
    /// with an ability that is on no card, and the engine offering it.
    fn lantern_land() -> (PlayerView, LegalActions) {
        let id = ObjectId::new(7, 0);
        let mut land = token(7, 0, "Mountain", 0, 0);
        land.types = baylee_core::types::TypeSet::LAND;
        land.power = None;
        land.toughness = None;
        land.granted_mana = Some(baylee_view::GrantedMana {
            colors: vec![
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green,
            ],
            amount: 1,
        });
        let view = ViewBuilder::new(2).with_battlefield(0, [land]).build();
        let legal = LegalActions {
            abilities: vec![(id, GRANTED_ABILITY)],
            mana_abilities: vec![id],
            ..LegalActions::default()
        };
        (view, legal)
    }

    /// The gap this closes. `GRANTED_ABILITY` is `u32::MAX`, so the registry
    /// lookup that reads every other ability finds nothing there — a land
    /// under a Lantern counted for zero and the player tapped it by hand.
    #[test]
    fn a_granted_mana_ability_is_a_source_the_planner_can_see() {
        let (view, legal) = lantern_land();
        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1, "one permanent, one source");
        let source = &sources[0];
        assert_eq!(source.amount, 1);
        assert_eq!(source.colors.len(), 5, "any colour, as the Lantern says");
        assert_eq!(
            source.tap,
            Tap::Ability(GRANTED_ABILITY),
            "tapped through the handle the engine offered it under"
        );
    }

    /// A real Mountain under the Lantern: both producers fire — the CR 305.6
    /// shortcut says red, the grant says any colour — and the permanent still
    /// taps once. Two entries would let the planner pay `{R}{U}` with one
    /// land, which is a plan the engine refuses after it is already tapped.
    #[test]
    fn a_basic_under_a_lantern_is_one_source_and_it_is_the_better_one() {
        let (mut view, legal) = lantern_land();
        view.battlefield[0]
            .subtypes
            .insert(baylee_core::generated::subtypes::land::MOUNTAIN);

        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1, "a land taps once, however many ways");
        assert_eq!(
            sources[0].colors.len(),
            5,
            "the grant wins: the Lantern makes this land strictly better"
        );
    }

    /// The same land with nothing granting it anything. Worth its own test
    /// because the failure it guards is the loud one: a source invented out
    /// of an empty field is a plan the engine refuses halfway through, with
    /// the lands already tapped.
    #[test]
    fn a_land_with_no_grant_offers_nothing() {
        let (mut view, legal) = lantern_land();
        view.battlefield[0].granted_mana = None;
        assert!(sources(&view, &legal).is_empty());
    }
}
