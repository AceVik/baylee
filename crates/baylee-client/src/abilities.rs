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
    //
    // `offered_as_mana` is which tap it claimed, so the list below does not
    // offer the same button again under its own name.
    let mut offered_as_mana = None;
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
            offered_as_mana = Some(source.tap);
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
        // Already offered above, whichever of them won. Two conditions
        // because they catch different things: a granted ability is on no
        // card, so `printed_source` has nothing to say about it, and a
        // printed one may have lost to the CR 305.6 shortcut and still must
        // not be listed a second time.
        if offered_as_mana == Some(Tap::Ability(index))
            || crate::manasources::printed_source(view, object, index).is_some()
        {
            continue;
        }
        let Some(action) = interaction.activate(object, index) else {
            continue;
        };
        // The synthetic indices are not positions on the card, so neither the
        // registry nor the card's ability list has anything to say about
        // them — and the fallback label counts them out as "Ability N", which
        // on `GRANTED_ABILITY` overflows: in a debug build the client dies the
        // moment a Chromatic Lantern's land is under the pointer, and in a
        // release build the button reads "Ability 0". A prepared cast is a
        // cast and never a mana ability; a granted one is whichever the view
        // said, per slot.
        let (label, mana) = if let Some(slot) = baylee_engine::choice::granted_slot(index) {
            (Phrase::GrantedAbility.text(lang).to_string(), {
                match view.object(object).and_then(|o| o.granted_mana.as_ref()) {
                    // Whether *this* grant is the mana one, not whether the
                    // permanent has one anywhere: Urza's Saga is granted a
                    // mana ability and a Construct ability, and
                    // `legal.mana_abilities` holds the Saga for the first.
                    Some(granted) => granted.slot == slot,
                    // The view could not reduce this grant to "n mana of
                    // these colours" — a `LandColor` source, say. The engine
                    // still answered the question, but per *permanent*, so
                    // it only settles anything where the permanent offers one
                    // grant. Where it offers several, the safe reading is
                    // "not a mana ability": that costs an extra tap, while
                    // the other way round fires a Construct with no arming.
                    //
                    // And the permanent's *own* mana ability has to be
                    // counted out first, because it is in the same list: a
                    // Mountain granted one non-mana ability is named there
                    // for CR 305.6, and reading that as the grant would fire
                    // the grant unarmed — the exact mistake this branch is
                    // written to avoid.
                    None => {
                        granted_mana_offers(view, legal, object) >= 1
                            && granted_count(legal, object) == 1
                    }
                }
            })
        } else if index == baylee_engine::choice::PREPARED_CAST {
            (Phrase::PreparedCast.text(lang).to_string(), false)
        } else {
            (
                printed_label(lang, view, object, index),
                makes_mana(view, object, index),
            )
        };
        out.push(AbilityOption {
            action,
            label,
            mana,
        });
    }
    out
}

/// How many of `object`'s entries in `legal.mana_abilities` are *grants*.
///
/// The engine names a permanent there once for a mana ability of its own —
/// printed, or the intrinsic one a basic land type carries (CR 305.6) — and
/// once more per granted mana ability. So its own has to be subtracted before
/// what is left can be read as grants at all.
///
/// The subtraction is deliberately unconditional on whether that own ability
/// is *currently* activatable: a permanent whose own ability is already spent
/// is named once for the grant alone, and counting it out anyway answers
/// "not a mana ability", which costs a tap instead of firing something
/// unarmed.
fn granted_mana_offers(
    view: &PlayerView,
    legal: &baylee_engine::choice::LegalActions,
    object: ObjectId,
) -> usize {
    let named = legal
        .mana_abilities
        .iter()
        .filter(|o| **o == object)
        .count();
    named.saturating_sub(usize::from(owns_a_mana_ability(view, object)))
}

/// Whether the permanent has a mana ability that is not a grant.
///
/// The subtypes are the projected ones, which is what CR 305.6 asks for: an
/// animated Mountain is still a Mountain and still taps for `{R}`.
///
/// Deliberately not `manaplan::basic_land_color`, which answers a different
/// question — it returns `None` for a Taiga, because *which* colour one tap
/// makes has no single answer there. What is asked here is whether the land
/// has an intrinsic mana ability at all, and a dual has two.
fn owns_a_mana_ability(view: &PlayerView, object: ObjectId) -> bool {
    use baylee_core::generated::subtypes::land;
    let Some(o) = view.object(object) else {
        return false;
    };
    if [
        land::PLAINS,
        land::ISLAND,
        land::SWAMP,
        land::MOUNTAIN,
        land::FOREST,
    ]
    .into_iter()
    .any(|t| o.subtypes.contains(t))
    {
        return true;
    }
    let Some(card) = o.card else {
        return false;
    };
    baylee_cards::by_index(card.index).is_some_and(|def| {
        def.abilities_for_face(card.face as usize).iter().any(|a| {
            matches!(
                a,
                AbilityDef::Activated {
                    mana_ability: true,
                    ..
                }
            )
        })
    })
}

/// How many granted abilities the engine is offering for `object` right now.
fn granted_count(legal: &baylee_engine::choice::LegalActions, object: ObjectId) -> usize {
    legal
        .abilities
        .iter()
        .filter(|(o, i)| *o == object && baylee_engine::choice::granted_slot(*i).is_some())
        .count()
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

        // Granted, the view has nothing to say about what it makes — and the
        // engine says it makes mana, so it is one tap, not arm-then-act.
        // That fallback is the only reading left when a grant cannot be
        // reduced to "n mana of these colours", and it is sound here because
        // the permanent is offering exactly one grant for it to be about.
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
    /// A Chromatic Lantern's land, as it actually arrives: the engine offers
    /// the grant *and* the view says what it makes. Both halves of the
    /// chooser then have something to say about the same tap, and for one
    /// commit they both said it — the player got two buttons that did the
    /// same thing, one of them labelled "Granted ability".
    #[test]
    fn a_granted_mana_ability_is_one_button_and_not_two() {
        let id = ObjectId::new(1, 0);
        let mut land = token(1, 0, "Mountain", 0, 0);
        land.types = baylee_core::types::TypeSet::LAND;
        land.power = None;
        land.toughness = None;
        land.granted_mana = Some(baylee_view::GrantedMana {
            slot: 0,
            colors: vec![ManaColor::Red],
            amount: 1,
        });
        let view = ViewBuilder::new(2).with_battlefield(0, [land]).build();

        let i = offering(vec![(id, GRANTED_ABILITY)], vec![id]);
        let out = options(Lang::En, &view, &i, id);
        assert_eq!(out.len(), 1, "one tap, one button: {out:?}");
        assert!(out[0].mana);
    }

    /// Urza's Saga is granted two abilities and only one of them makes mana.
    /// Reading "is this a mana ability" off `legal.mana_abilities` — which
    /// holds the *permanent*, not the slot — marked the Construct ability as
    /// one tap, which would have sent it with no arming and no confirmation.
    #[test]
    fn a_second_grant_is_not_a_mana_ability_because_the_first_one_is() {
        let id = ObjectId::new(1, 0);
        let mut saga = token(1, 0, "Urza's Saga", 0, 0);
        saga.types = baylee_core::types::TypeSet::LAND;
        saga.power = None;
        saga.toughness = None;
        saga.granted_mana = Some(baylee_view::GrantedMana {
            slot: 0,
            colors: vec![ManaColor::Colorless],
            amount: 1,
        });
        let view = ViewBuilder::new(2).with_battlefield(0, [saga]).build();

        let i = offering(
            vec![
                (id, baylee_engine::choice::granted_ability(0)),
                (id, baylee_engine::choice::granted_ability(1)),
            ],
            vec![id],
        );
        let out = options(Lang::En, &view, &i, id);
        assert_eq!(out.len(), 2, "two grants, two buttons: {out:?}");
        let construct = out
            .iter()
            .find(|o| {
                o.action
                    == PlayerAction::ActivateAbility {
                        source: id,
                        ability_index: baylee_engine::choice::granted_ability(1),
                    }
            })
            .expect("chapter II is offered");
        assert!(
            !construct.mana,
            "the permanent has a granted mana ability; this is not it"
        );
    }

    /// The same fallback with two grants on the permanent, which is where it
    /// stops being sound: `legal.mana_abilities` names the permanent, so it
    /// cannot say *which* of them makes the mana. Reading it anyway would
    /// have fired the other one on a single tap, with no arming and no way
    /// back — and arming a mana ability by mistake only ever costs a tap.
    #[test]
    fn an_unreadable_grant_is_not_a_mana_ability_when_there_are_two_of_them() {
        let id = ObjectId::new(1, 0);
        let view = ViewBuilder::new(2)
            .with_battlefield(0, [token(1, 0, "Ally", 2, 2)])
            .build();
        let i = offering(
            vec![
                (id, baylee_engine::choice::granted_ability(0)),
                (id, baylee_engine::choice::granted_ability(1)),
            ],
            vec![id],
        );
        let out = options(Lang::En, &view, &i, id);
        assert_eq!(out.len(), 2, "two grants, two buttons: {out:?}");
        assert!(
            out.iter().all(|o| !o.mana),
            "neither can be claimed as the mana one: {out:?}"
        );
    }

    /// The other way that fallback goes wrong, and the one a real card
    /// reaches first: a *basic land* is named in `legal.mana_abilities` for
    /// its own intrinsic mana (CR 305.6), whatever it was granted. Counting
    /// that entry as evidence about the grant marks a non-mana grant as one
    /// tap, and it fires with no arming.
    ///
    /// No card in the pool grants a land a non-mana ability today — Lantern,
    /// Saga and Guide all grant mana — so this guards the reading rather
    /// than a card, which is why it is written from both sides.
    #[test]
    fn a_basics_own_mana_says_nothing_about_a_grant_that_cannot_be_read() {
        use baylee_core::generated::subtypes::land;
        let id = ObjectId::new(1, 0);
        let mut mountain = token(1, 0, "Mountain", 0, 0);
        mountain.types = baylee_core::types::TypeSet::LAND;
        mountain.subtypes = baylee_core::types::SubtypeSet::from_slice(&[land::MOUNTAIN]);
        mountain.power = None;
        mountain.toughness = None;
        // The view could not reduce the grant to "n mana of these colours".
        mountain.granted_mana = None;
        let view = ViewBuilder::new(2).with_battlefield(0, [mountain]).build();

        // The land really does have two things to do — its own `{R}` and the
        // grant — so both are drawn. What matters is which of them is one
        // tap: the intrinsic is, the unreadable grant is not.
        let granted = |legal: Vec<ObjectId>| {
            let i = offering(vec![(id, GRANTED_ABILITY)], legal);
            let out = options(Lang::En, &view, &i, id);
            assert_eq!(out.len(), 2, "the intrinsic and the grant: {out:?}");
            assert!(
                out.iter().any(|o| o.mana
                    && o.action == PlayerAction::ActivateManaAbility { source: id }),
                "the Mountain still taps for `{{R}}`: {out:?}"
            );
            out.into_iter()
                .find(|o| {
                    o.action
                        == PlayerAction::ActivateAbility {
                            source: id,
                            ability_index: GRANTED_ABILITY,
                        }
                })
                .expect("the grant is offered")
        };

        // Named once, and its own `{R}` is what that entry is.
        assert!(
            !granted(vec![id]).mana,
            "the Mountain's own mana is not evidence about the grant"
        );

        // Named twice: its own, and a granted one. Now there is an entry
        // left over and exactly one grant it can be about.
        assert!(
            granted(vec![id, id]).mana,
            "one entry over and one grant to be about"
        );
    }
}
