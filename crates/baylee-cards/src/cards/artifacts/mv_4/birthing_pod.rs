//! Birthing Pod — {3}{G/P} — Artifact
//! Oracle: ({G/P} can be paid with either {G} or 2 life.)
//! Oracle: {1}{G/P}, {T}, Sacrifice a creature: Search your library for a creature card with mana value equal to 1 plus the sacrificed creature's mana value, put that card onto the battlefield, then shuffle. Activate only as a sorcery.
//! Set: NPH #104 — New Phyrexia | Scryfall ID: b768efa2-e56b-4a7e-ace8-d673f10e0714 | Oracle ID: f8b9dd54-0837-47f4-ad14-7a0322d46d5f
// PARTIAL — the activation sacrifices a creature, writes its mana value on
// the ability, and the search reads it back as an exact bound. The {G/P} in
// the activation cost is paid with {G} only.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BIRTHING_POD,
    oracle_id = "f8b9dd54-0837-47f4-ad14-7a0322d46d5f",
    scryfall_id = "b768efa2-e56b-4a7e-ace8-d673f10e0714",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Birthing Pod",
        mana_cost = mana!("{3}{G/P}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "\"{G/P} can be paid with either {G} or 2 life\" — an activation cost \
         pays Phyrexian mana with mana only; the life option is the cast \
         wizard's and no activation asks it"
    ),
    // NOT SUPPORTED: paying the activation's {G/P} with 2 life. The cast
    // wizard offers the life for a spell's Phyrexian symbols; an activation
    // pays its mana part through `mana_pay`, which pays Phyrexian with mana.
    abilities = &[activated!(
        cost!("{1}{G/P}", TapSelf, Sacrifice(&Filter::CREATURE)),
        &[Effect::SearchLibraryOf {
            library: PlayerRel::You,
            owner_searches: true,
            filter: &Filter::CREATURE,
            mana_value: Some(ManaValueBound {
                cmp: ManaValueCmp::Exactly,
                amount: Amount::Plus {
                    base: &Amount::SacrificedManaValue,
                    offset: 1,
                },
            }),
            finds: &[Find::BATTLEFIELD],
            optional: false,
        }],
        timing = ActivationTiming::SorcerySpeed,
    )],
);
