//! Agatha's Soul Cauldron — {2} — Legendary Artifact
//! Oracle: You may spend mana as though it were mana of any color to activate abilities of creatures you control.
//! Oracle: Creatures you control with +1/+1 counters on them have all activated abilities of all creature cards exiled with Agatha's Soul Cauldron.
//! Oracle: {T}: Exile target card from a graveyard. When a creature card is exiled this way, put a +1/+1 counter on target creature you control.
//! Set: WOE #242 — Wilds of Eldraine | Scryfall ID: 019b51b0-e5c6-4208-922b-7736686dddcd | Oracle ID: c259e16f-2a44-4552-8678-815f757a02e8
// PARTIAL — the {T} ability: exile a target card from a graveyard. The two
// other printed abilities and the reflexive counter trigger have no DSL
// spelling; each carries a NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;

/// "Target card from a graveyard" — any graveyard, named once for the target
/// requirement and once for the effect that exiles it.
const GRAVEYARD_CARD: TargetSpec = TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer);

card!(
    index = index::AGATHA_S_SOUL_CAULDRON,
    oracle_id = "c259e16f-2a44-4552-8678-815f757a02e8",
    scryfall_id = "019b51b0-e5c6-4208-922b-7736686dddcd",
    faces = &[face!(
        name = "Agatha's Soul Cauldron",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "the mana permission is scoped to creatures' activated abilities, the \
         static ability grants the activated abilities of cards exiled with \
         the source, and the counter is a reflexive trigger on that exile",
    ),
    abilities = &[
        // NOT SUPPORTED: "You may spend mana as though it were mana of any
        // color to activate abilities of creatures you control."
        // `Modifier::ManaIsAnyColor` carries no scope, so writing it would
        // spend off-color mana on every spell and every ability rather than
        // on a creature's activated ability alone.
        // NOT SUPPORTED: "Creatures you control with +1/+1 counters on them
        // have all activated abilities of all creature cards exiled with
        // Agatha's Soul Cauldron." No `Filter` asks an object for a counter,
        // and `Modifier::GrantActivated` grants one printed cost and effect
        // pair rather than the abilities of the cards the source exiled.
        // NOT SUPPORTED: "When a creature card is exiled this way, put a
        // +1/+1 counter on target creature you control." A reflexive trigger
        // on the exile below — `Trigger::ExiledFromBattlefield` is another
        // zone, and nothing links an event back to "this way".
        activated!(
            Cost::TAP,
            &[Effect::exile(GRAVEYARD_CARD)],
            target = Some(GRAVEYARD_CARD),
        ),
    ],
);
