//! Search for Azcanta // Azcanta, the Sunken Ruin — {1}{U} — Legendary Enchantment // Legendary Land
//! Oracle: At the beginning of your upkeep, surveil 1. Then if you have seven or more cards in your graveyard, you may transform Search for Azcanta. (Look at the top card of your library. You may put that card into your graveyard.)
//! Oracle: (Transforms from Search for Azcanta.)
//! Oracle: {T}: Add {U}.
//! Oracle: {2}{U}, {T}: Look at the top four cards of your library. You may reveal a noncreature, nonland card from among them and put it into your hand. Put the rest on the bottom of your library in any order.
//! Set: XLN #74 — Ixalan | Scryfall ID: 1a7e242e-bb48-4134-a1c2-6033713d658f | Oracle ID: f74c4d96-bc4a-4d32-9519-a753d192144e
//! Face: Search for Azcanta — {1}{U} — Legendary Enchantment
//! Face: Azcanta, the Sunken Ruin —  — Legendary Land
// PARTIAL — the upkeep "surveil 1" and the back face's "{T}: Add {U}" are
// built; the transform clause and the back face's card-selection ability are
// not expressible.

use baylee_cards_dsl::prelude::*;

/// The back face's mana ability. Azcanta prints no basic land type, so
/// CR 305.6 grants it nothing and the card has to say `{T}: Add {U}` itself.
static BACK_ABILITIES: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])];

card!(
    index = index::SEARCH_FOR_AZCANTA,
    oracle_id = "f74c4d96-bc4a-4d32-9519-a753d192144e",
    scryfall_id = "1a7e242e-bb48-4134-a1c2-6033713d658f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Search for Azcanta",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        // NOT SUPPORTED: "{2}{U}, {T}: Look at the top four cards of your
        // library. You may reveal a noncreature, nonland card from among them
        // and put it into your hand. Put the rest on the bottom of your
        // library in any order." — Effect::LookAtTopPick takes only a count
        // and a pick, so the reveal could not be restricted to a noncreature,
        // nonland card.
        face!(
            name = "Azcanta, the Sunken Ruin",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = BACK_ABILITIES,
        ),
    ],
    coverage = Coverage::Partial(
        "the upkeep trigger's transform clause is gated on seven or more cards in \
         your graveyard and no Condition or Effect reads the size of your own \
         graveyard; the back face's look-at-four ability has no LookAtTopPick \
         filter and so cannot be restricted to a noncreature, nonland card"
    ),
    abilities = &[
        // NOT SUPPORTED: "Then if you have seven or more cards in your
        // graveyard, you may transform Search for Azcanta." — the transform is
        // gated on a graveyard count nothing in the DSL can ask, so it is not
        // written at all rather than written unconditionally.
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::surveil(1)]
        ),
    ],
);
