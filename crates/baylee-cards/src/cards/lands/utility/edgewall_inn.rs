//! Edgewall Inn — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: As this land enters, choose a color.
//! Oracle: {T}: Add one mana of the chosen color.
//! Oracle: {3}, {T}, Sacrifice this land: Return target card that has an Adventure from your graveyard to your hand.
//! Set: WOE #255 — Wilds of Eldraine | Scryfall ID: ec435e54-628a-43bd-8804-cbc37e375bce | Oracle ID: 4dfd33e8-7e30-493b-8564-f7df5f0257aa
// PARTIAL — enters tapped, asks for a color as it enters, and taps for the
// color it was given; the fourth line has no filter that can name a card
// "that has an Adventure" and is left off (see the abilities below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EDGEWALL_INN,
    oracle_id = "4dfd33e8-7e30-493b-8564-f7df5f0257aa",
    scryfall_id = "ec435e54-628a-43bd-8804-cbc37e375bce",
    faces = &[face!(
        name = "Edgewall Inn",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped, EnterModifier::ChooseColor],
    )],
    coverage = Coverage::Partial("no Filter variant matches a card \"that has an Adventure\""),
    // NOT SUPPORTED: "{3}, {T}, Sacrifice this land: Return target card that
    // has an Adventure from your graveyard to your hand." — the first three
    // lines are the land itself and are built below; this one needs a filter
    // for a printed characteristic no Filter variant carries, and any wider
    // filter (Filter::Any) would let the ability return a card it may not.
    abilities = &[mana_ability!(&[Effect::mana_chosen()])],
);
