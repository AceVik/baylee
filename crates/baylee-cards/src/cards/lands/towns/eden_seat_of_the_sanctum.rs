//! Eden, Seat of the Sanctum — (no cost) — Land — Town
//! Oracle: {T}: Add {C}.
//! Oracle: {5}, {T}: Mill two cards. Then you may sacrifice this land. When you do, return another target permanent card from your graveyard to your hand.
//! Set: FIN #277 — Final Fantasy | Scryfall ID: e28eac1e-adc7-4f8d-b206-bef09ba07d38 | Oracle ID: 84856b92-5ce8-47f3-9a1c-78d6a3e26aca
// PARTIAL — the mana ability is built; the {5}, {T} ability is not, see the
// NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EDEN_SEAT_OF_THE_SANCTUM,
    oracle_id = "84856b92-5ce8-47f3-9a1c-78d6a3e26aca",
    scryfall_id = "e28eac1e-adc7-4f8d-b206-bef09ba07d38",
    faces = &[face!(
        name = "Eden, Seat of the Sanctum",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::TOWN],
    ),],
    coverage = Coverage::Partial(
        "{5}, {T} ability: needs a reflexive when-you-do trigger, which the DSL does not have"
    ),
    abilities = &[
        // NOT SUPPORTED: "{5}, {T}: Mill two cards. Then you may sacrifice
        // this land. When you do, return another target permanent card from
        // your graveyard to your hand." — the return sits on a reflexive
        // triggered ability (CR 603.12), and `Trigger` has no variant for
        // "you sacrificed this" (its events are Enter/Leave/Die/Cast/Target/
        // Exile/CombatDamage/BecomesTapped/NthSpell/Draw/Attack/Step). The
        // nearest shape, one `MayDo` wrapping `SacrificeSelf` beside
        // `GraveyardToHand`, is a different card rather than a shorter
        // spelling of this one: the target would be chosen as the ability is
        // activated — before the mill, and with no way to return a card the
        // mill just put there — and an activation carrying a required
        // graveyard target cannot happen at all on an empty graveyard, so a
        // player could not even pay {5}, {T} to mill two.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
