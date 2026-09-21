//! Sidequest: Catch a Fish // Cooking Campsite — {2}{W} — Enchantment // Land
//! Oracle: At the beginning of your upkeep, look at the top card of your library. If it's an artifact or creature card, you may reveal it and put it into your hand. If you put a card into your hand this way, create a Food token and transform this enchantment.
//! Oracle: {T}: Add {W}.
//! Oracle: {3}, {T}, Sacrifice an artifact: Put a +1/+1 counter on each creature you control. Activate only as a sorcery.
//! Set: FIN #31 — Final Fantasy | Scryfall ID: bdb5452e-d97f-409b-91d0-2664f39b09b8 | Oracle ID: bd7c328e-0380-46f8-bb85-7bf4e201b7ac
//! Face: Sidequest: Catch a Fish — {2}{W} — Enchantment
//! Face: Cooking Campsite —  — Land
// IMPLEMENTED — the back face, Cooking Campsite: {T}: Add {W}, and {3}, {T}, Sacrifice an
// artifact: put a +1/+1 counter on each creature you control, at sorcery speed. The front
// face stays unbuilt, so the card is `Partial`; see the NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "At the beginning of your upkeep, look at the top card of your library. If
// it's an artifact or creature card, you may reveal it and put it into your hand. If you put
// a card into your hand this way, create a Food token and transform this enchantment." — no
// `Effect` reads the top card of a library through a `Filter` (`LookAtTopPick` takes no filter
// and bottoms whatever it does not take; `SearchLibrary` is a search and a shuffle), and no
// `Effect` branches on whether an earlier effect of the same resolution put a card into a
// hand — so the transform has no trigger to hang on either.

/// Cooking Campsite's abilities, which are the back face's: a mana ability,
/// and a sorcery-speed artifact sacrifice that grows the team.
static BACK_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
    activated!(
        cost!("{3}", TapSelf, Sacrifice(&Filter::ARTIFACT)),
        &[Effect::AddCounterFilter {
            filter: &Filter::YOUR_CREATURE,
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        timing = ActivationTiming::SorcerySpeed,
    ),
];

card!(
    index = index::SIDEQUEST_CATCH_A_FISH,
    oracle_id = "bd7c328e-0380-46f8-bb85-7bf4e201b7ac",
    scryfall_id = "bdb5452e-d97f-409b-91d0-2664f39b09b8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Sidequest: Catch a Fish",
            mana_cost = mana!("{2}{W}"),
            types = TypeSet::ENCHANTMENT,
        ),
        face!(
            name = "Cooking Campsite",
            types = TypeSet::LAND,
            castable_from_hand = false,
            abilities = BACK_ABILITIES,
        ),
    ],
    coverage = Coverage::Partial(
        "front face: no Effect reads the top card of a library through a filter, and none \
         branches on whether an earlier effect put a card into a hand",
    ),
);
