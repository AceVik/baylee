//! Westvale Abbey // Ormendahl, Profane Prince — (no cost) — Land // Legendary Creature — Demon
//! Oracle: {T}: Add {C}.
//! Oracle: {5}, {T}, Pay 1 life: Create a 1/1 white and black Human Cleric creature token.
//! Oracle: {5}, {T}, Sacrifice five creatures: Transform this land, then untap it.
//! Oracle: Flying, lifelink, indestructible, haste
//! Set: INR #287 — Innistrad Remastered | Scryfall ID: 5fbc6091-a161-45b0-9932-543b569caaee | Oracle ID: 04eeb9ad-5c59-411b-8809-db8349838588
//! Face: Westvale Abbey —  — Land
//! Face: Ormendahl, Profane Prince —  — Legendary Creature — Demon
// PARTIAL — {T}: Add {C}, the Human Cleric line and Ormendahl's printed
// keyword line; the transform is not expressible (see NOT SUPPORTED below).

use crate::generated_tokens;
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
        "\"Sacrifice five creatures: Transform this land\": no effect transforms \
         a permanent in place",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{5}", TapSelf, PayLife(1)),
            &[Effect::CreateToken {
                token: &generated_tokens::HUMAN_CLERIC_1_1_WHITE_BLACK
            }]
        ),
        // NOT SUPPORTED: "{5}, {T}, Sacrifice five creatures: Transform this
        // land, then untap it." — the cost is sayable (five
        // `Sacrifice(&Filter::YOUR_CREATURE)` parts), but no effect
        // transforms a permanent in place: `Effect::ExileSelfReturnAsFace`
        // exiles it and returns a new object (CR 400.7), which is not how
        // CR 701.27a transforms — the permanent would lose its counters, its
        // attachments and "then untap it" would read the wrong object.
    ],
);
