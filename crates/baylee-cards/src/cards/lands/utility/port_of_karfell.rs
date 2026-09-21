//! Port of Karfell — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {3}{U}{B}{B}, {T}, Sacrifice this land: Mill four cards, then return a creature card from your graveyard to the battlefield tapped. (To mill a card, put the top card of your library into your graveyard.)
//! Set: MKC #280 — Murders at Karlov Manor Commander | Scryfall ID: c1377ca1-aa54-4fed-b289-45309eb2f6b6 | Oracle ID: f500a8a4-6135-448c-9116-bd2695a72229
// IMPLEMENTED — the land enters tapped, taps for {U}, and its last ability
// mills four cards and reanimates a creature card. The returned creature
// comes back untapped; see the NOT SUPPORTED line below, which is why the
// coverage is Partial.
// NOT SUPPORTED: "…return a creature card from your graveyard to the
// battlefield tapped." — `Effect::GraveyardToBattlefield` has no tapped
// destination, and no other variant puts a chosen graveyard card onto the
// battlefield.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PORT_OF_KARFELL,
    oracle_id = "f500a8a4-6135-448c-9116-bd2695a72229",
    scryfall_id = "c1377ca1-aa54-4fed-b289-45309eb2f6b6",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Port of Karfell",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "the reanimated creature enters untapped — Effect::GraveyardToBattlefield \
         has no destination modifier"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        // NOT SUPPORTED: "…to the battlefield tapped" — the card comes back
        // untapped, because `Effect::GraveyardToBattlefield` only names what
        // it moves and not how it arrives.
        activated!(
            cost!("{3}{U}{B}{B}", TapSelf, SacrificeSelf),
            &[
                Effect::Mill {
                    amount: Amount::Fixed(4),
                    target: PlayerRel::You,
                },
                Effect::GraveyardToBattlefield {
                    target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
                },
            ],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::You
            )),
        ),
    ],
);
