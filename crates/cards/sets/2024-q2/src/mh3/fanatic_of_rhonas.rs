//! Fanatic of Rhonas — {1}{G} — Creature — Snake Druid
//! Oracle: {T}: Add {G}.
//! Oracle: Ferocious — {T}: Add {G}{G}{G}{G}. Activate only if you control a creature with power 4 or greater.
//! Oracle: Eternalize {2}{G}{G} ({2}{G}{G}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a 4/4 black Zombie Snake Druid with no mana cost. Eternalize only as a sorcery.)
//! Set: MH3 #152 — Modern Horizons 3 | Scryfall ID: 1f9fb33a-3b39-4aff-93b8-aedafe0ea694 | Oracle ID: 7973820b-fdaf-46ec-9e3e-d4c0e77b5067

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FANATIC_OF_RHONAS,
    oracle_id = "7973820b-fdaf-46ec-9e3e-d4c0e77b5067",
    scryfall_id = "1f9fb33a-3b39-4aff-93b8-aedafe0ea694",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Fanatic of Rhonas",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(4),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // Ferocious — "Activate only if you control a creature with power 4
        // or greater."
        mana_ability!(
            Cost::TAP,
            &[Effect::mana(ManaColor::Green, 4)],
            condition = Some(Condition::ControlCount(
                &Filter::YOUR_CREATURE_WITH_POWER_4_OR_GREATER,
                1
            )),
        ),
        // Eternalize {2}{G}{G} (CR 702.129a): "{2}{G}{G}, Exile this card
        // from your graveyard: Create a token that's a copy of it, except
        // it's a 4/4 black Zombie Snake Druid with no mana cost. Eternalize
        // only as a sorcery." The copy is of the card, so Snake Druid comes
        // with it and Zombie is added.
        activated!(
            cost!("{2}{G}{G}", ExileSelf),
            &[Effect::CreateTokenCopyOfSource {
                mods: &[
                    CopyMod::SetPT(4, 4),
                    CopyMod::SetColor(ColorSet::from_slice(&[Color::Black])),
                    CopyMod::AddSubtype(subtypes::creature::ZOMBIE),
                    CopyMod::NoManaCost,
                ],
            }],
            zone = ActivationZone::Graveyard,
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
