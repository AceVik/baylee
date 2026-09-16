//! Golgari Thug — {1}{B} — Creature — Human Warrior
//! Oracle: When this creature dies, put target creature card from your graveyard on top of your library.
//! Oracle: Dredge 4 (If you would draw a card, you may mill four cards instead. If you do, return this card from your graveyard to your hand.)
//! Set: RVR #76 — Ravnica Remastered | Scryfall ID: 6a8c5d3b-71b4-4e99-82e9-dfc0b98698b0 | Oracle ID: a426a258-fd8b-489c-8642-9868ee47de85
// PARTIAL — a {1}{B} 1/1 whose death puts a targeted creature card out of
// your graveyard back on top of your library; the printed dredge never
// replaces a draw, so the card only ever comes back by being drawn.
// NOT SUPPORTED: `Dredge 4` — "If you would draw a card, you may mill four
// cards instead. If you do, return this card from your graveyard to your
// hand." Nothing in the DSL says "instead of drawing". `AbilityDef::Replacement`
// takes a `ReplacementRule`, whose four variants replace token creation,
// counter placement and trigger counts and none of them a draw;
// `Trigger::Draws(PlayerRel::You)` is a trigger that fires *after* the card is
// drawn and cannot stop it. Nor can an ability of this card function where
// dredge does: `ActivationZone` is `Battlefield` or `Hand`, so no ability is
// live in a graveyard, and `Modifier::GrantsFlashback` is the one graveyard
// permission in the vocabulary — it grants casting, not a replaced draw.
// Dredge is also no `KeywordSet` bit: the `keywords!` table has no `DREDGE`
// and `keyword_tests::ENFORCED` does not list it. The card therefore plays
// exactly as though the line were not printed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOLGARI_THUG,
    oracle_id = "a426a258-fd8b-489c-8642-9868ee47de85",
    scryfall_id = "6a8c5d3b-71b4-4e99-82e9-dfc0b98698b0",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Golgari Thug",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial("dredge 4 is not written: a draw is never replaced"),
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::GraveyardToTop {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        )))
    )],
);

// Engine-level test belongs in baylee-engine (card_tests), beside Myr
// Retriever's: the printed sentence says no "another", so the Thug lying in
// its own graveyard is a legal target for its own trigger — the one thing
// this card asks that Myr Retriever's `another` forbids.
