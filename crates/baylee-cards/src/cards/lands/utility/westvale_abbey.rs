//! Westvale Abbey // Ormendahl, Profane Prince — (no cost) — Land // Legendary Creature — Demon
//! Oracle: {T}: Add {C}.
//! Oracle: {5}, {T}, Pay 1 life: Create a 1/1 white and black Human Cleric creature token.
//! Oracle: {5}, {T}, Sacrifice five creatures: Transform this land, then untap it.
//! Oracle: Flying, lifelink, indestructible, haste
//! Set: INR #287 — Innistrad Remastered | Scryfall ID: 5fbc6091-a161-45b0-9932-543b569caaee | Oracle ID: 04eeb9ad-5c59-411b-8809-db8349838588
//! Face: Westvale Abbey —  — Land
//! Face: Ormendahl, Profane Prince —  — Legendary Creature — Demon
// PARTIAL — {T}: Add {C} and Ormendahl's printed keyword line; the two
// {5}, {T} abilities are not expressible (see the NOT SUPPORTED lines below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WESTVALE_ABBEY,
    oracle_id = "04eeb9ad-5c59-411b-8809-db8349838588",
    scryfall_id = "5fbc6091-a161-45b0-9932-543b569caaee",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(name = "Westvale Abbey", types = TypeSet::LAND,),
        face!(
            name = "Ormendahl, Profane Prince",
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::DEMON],
            power = Some(9),
            toughness = Some(7),
            // A back face states only what it prints (CR 702.145a's rule read
            // the other way round): Ormendahl's four keywords, none of them
            // shared with the land side.
            keywords = KeywordSet::FLYING
                .union(KeywordSet::LIFELINK)
                .union(KeywordSet::INDESTRUCTIBLE)
                .union(KeywordSet::HASTE),
            castable_from_hand = false,
        ),
    ],
    coverage = Coverage::Partial(
        "the two {5}, {T} abilities are not expressible: there is no 1/1 white \
         and black Human Cleric token in crate::tokens, and CostPart::Sacrifice \
         names one permanent and carries no count for \"sacrifice five creatures\"",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{5}, {T}, Pay 1 life: Create a 1/1 white and black
        // Human Cleric creature token." — `Effect::CreateToken` needs a
        // `&'static TokenDef` from `crate::tokens` (the index into the ledger
        // IS the token's art id), and no such token is in it; a card file may
        // not define one of its own.
        // NOT SUPPORTED: "{5}, {T}, Sacrifice five creatures: Transform this
        // land, then untap it." — `CostPart::Sacrifice(filter)` names one
        // permanent and has no count, so the printed five cannot be said. (The
        // effect half would be `Effect::ExileSelfReturnAsFace { face: 1 }` then
        // `Effect::UntapSelf`.)
    ],
);
