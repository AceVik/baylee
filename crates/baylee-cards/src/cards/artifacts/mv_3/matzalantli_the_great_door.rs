//! Matzalantli, the Great Door // The Core — {3} — Legendary Artifact // Legendary Land
//! Oracle: {T}: Draw a card, then discard a card.
//! Oracle: {4}, {T}: Transform Matzalantli. Activate only if there are four or more permanent types among cards in your graveyard. (Artifact, battle, creature, enchantment, land, and planeswalker are permanent types.)
//! Oracle: (Transforms from Matzalantli.)
//! Oracle: Fathomless descent — {T}: Add X mana of any one color, where X is the number of permanent cards in your graveyard.
//! Set: LCI #256 — The Lost Caverns of Ixalan | Scryfall ID: b4c31b29-06ba-436d-a3d9-18f4796c39be | Oracle ID: 16182e01-22ff-4786-985d-919b47c4aa4d
//! Face: Matzalantli, the Great Door — {3} — Legendary Artifact
//! Face: The Core —  — Legendary Land
// PARTIAL — built: {T} draw-then-discard on the front, fathomless descent on
// the back. Not built: the {4}, {T} transform, whose printed activation
// condition the DSL cannot state (see the NOT SUPPORTED line below).

use baylee_cards_dsl::prelude::*;

/// Fathomless descent — "{T}: Add X mana of any one color, where X is the
/// number of permanent cards in your graveyard."
///
/// One pick of X mana, which is `mana_choice_dynamic` and not
/// `mana_combination`: "any **one** colour" is a single colour chosen for the
/// whole amount. "Permanent card" is the six permanent types the card's own
/// reminder text lists — `HasType` asks `intersects`, so one union is one
/// clause — and the zone selector says *whose* graveyard is counted.
static FATHOMLESS_DESCENT: &[AbilityDef] = &[mana_ability!(&[Effect::mana_choice_dynamic(
    ALL_MANA_COLORS,
    Amount::CountOf {
        filter: &Filter::HasType(
            TypeSet::ARTIFACT
                .union(TypeSet::BATTLE)
                .union(TypeSet::CREATURE)
                .union(TypeSet::ENCHANTMENT)
                .union(TypeSet::LAND)
                .union(TypeSet::PLANESWALKER),
        ),
        zone: ZoneSel::GraveyardYou,
    },
)])];

card!(
    index = index::MATZALANTLI_THE_GREAT_DOOR,
    oracle_id = "16182e01-22ff-4786-985d-919b47c4aa4d",
    scryfall_id = "b4c31b29-06ba-436d-a3d9-18f4796c39be",
    faces = &[
        face!(
            name = "Matzalantli, the Great Door",
            mana_cost = mana!("{3}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "The Core",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = FATHOMLESS_DESCENT,
        ),
    ],
    coverage = Coverage::Partial(
        "the {4}, {T} transform clause: no Condition variant counts the permanent types among the cards in your graveyard, and an ability written without one would transform at any time",
    ),
    abilities = &[
        activated!(
            Cost::TAP,
            &[
                Effect::draw(1),
                Effect::DiscardForPlayers {
                    who: PlayerRel::You,
                    count: 1,
                },
            ],
        ),
        // NOT SUPPORTED: "{4}, {T}: Transform Matzalantli. Activate only if
        // there are four or more permanent types among cards in your
        // graveyard." The transform itself is sayable
        // (`Effect::ExileSelfReturnAsFace`), the printed restriction is not:
        // the five `Condition`s count permanents on the battlefield, counters
        // on the source, or the size of an opponent's graveyard, and none of
        // them counts *types* among the cards in your own. Written without it
        // the ability would transform whenever {4}, {T} could be paid, so it
        // comes off the card — a partial card is dealt into real decks.
    ],
);
