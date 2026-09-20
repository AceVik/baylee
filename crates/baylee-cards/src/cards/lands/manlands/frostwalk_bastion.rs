//! Frostwalk Bastion — (no cost) — Snow Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{S}: Until end of turn, this land becomes a 2/3 Construct artifact creature. It's still a land. ({S} can be paid with one mana from a snow source.)
//! Oracle: Whenever this land deals combat damage to a creature, tap that creature and it doesn't untap during its controller's next untap step.
//! Set: MH1 #240 — Modern Horizons | Scryfall ID: bfb5d83f-2b7c-4c22-ac37-938e7cd1654a | Oracle ID: ae4a18ec-70a3-4d21-b9e5-b13ab4901600
// PARTIAL — {T} for {C}, and the {1}{S} animation as three continuous
// effects on the source (type 4, subtype 4, P/T 7b); the combat-damage
// trigger has no DSL trigger and is dropped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::FROSTWALK_BASTION,
    oracle_id = "ae4a18ec-70a3-4d21-b9e5-b13ab4901600",
    scryfall_id = "bfb5d83f-2b7c-4c22-ac37-938e7cd1654a",
    faces = &[face!(
        name = "Frostwalk Bastion",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::SNOW,
    ),],
    coverage = Coverage::Partial(
        "the combat-damage-to-a-creature trigger is inexpressible: Trigger only has \
         DealsCombatDamageToPlayer, which names a player and not a creature",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}{S}"),
            &[
                // "becomes a … artifact creature. It's still a land" — types
                // are added, never removed, so the land type survives.
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE.union(TypeSet::ARTIFACT)),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::CONSTRUCT),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(2, 3),
                    Duration::UntilEndOfTurn,
                ),
            ]
        ),
    ],
);

// NOT SUPPORTED: "Whenever this land deals combat damage to a creature, tap
// that creature and it doesn't untap during its controller's next untap
// step." — `Trigger` has no combat-damage-to-a-creature variant
// (`DealsCombatDamageToPlayer` names a player), and the rider's duration is
// keyed to the damaged creature's controller, which
// `Duration::UntilYourNextUntapStep` does not say.
