//! Fiend Artisan — {B/G}{B/G} — Creature — Nightmare
//! Oracle: This creature gets +1/+1 for each creature card in your graveyard.
//! Oracle: {X}{B/G}, {T}, Sacrifice another creature: Search your library for a creature card with mana value X or less, put it onto the battlefield, then shuffle. Activate only as a sorcery.
//! Set: IKO #220 — Ikoria: Lair of Behemoths | Scryfall ID: 6cd9d800-6d31-42e2-87d2-772db0ff95ed | Oracle ID: 43b8456a-3333-4936-a09c-324327619c36
// IMPLEMENTED — +1/+1 for each creature card in your graveyard, and the
// sorcery-speed tutor: {X}{B/G}, {T}, sacrifice another creature, a creature
// card with mana value X or less onto the battlefield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FIEND_ARTISAN,
    oracle_id = "43b8456a-3333-4936-a09c-324327619c36",
    scryfall_id = "6cd9d800-6d31-42e2-87d2-772db0ff95ed",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Fiend Artisan",
        mana_cost = mana!("{B/G}{B/G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // "This creature gets +1/+1 for each creature card in your graveyard."
        static_ability!(
            Filter::This,
            Modifier::ModifyPTPerGraveyardCard {
                filter: &Filter::CREATURE,
                p: 1,
                t: 1,
            }
        ),
        // "{X}{B/G}, {T}, Sacrifice another creature: Search your library for
        // a creature card with mana value X or less, put it onto the
        // battlefield, then shuffle. Activate only as a sorcery."
        activated!(
            cost!(
                "{X}{B/G}",
                TapSelf,
                Sacrifice(&Filter::ANOTHER_CREATURE_YOU_CONTROL)
            ),
            &[Effect::SearchLibraryOf {
                library: PlayerRel::You,
                owner_searches: true,
                filter: &Filter::CREATURE,
                mana_value: Some(ManaValueBound {
                    cmp: ManaValueCmp::AtMost,
                    amount: Amount::X,
                }),
                finds: &[Find::BATTLEFIELD],
                optional: false,
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
