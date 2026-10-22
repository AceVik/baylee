//! Nature's Kiss — {1}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: {1}, Exile the top card of your graveyard: Enchanted creature gets +1/+1 until end of turn.
//! Set: WTH #136 — Weatherlight | Scryfall ID: 64b09c44-d463-45a9-9fa2-89407c21200b | Oracle ID: e3e87fee-5717-4ca1-adfd-4df8732020f8
// PARTIAL — Exiles any card from your graveyard to pay its activation cost; requiring the exiled card to be the top card of your graveyard is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NATURE_S_KISS,
    oracle_id = "e3e87fee-5717-4ca1-adfd-4df8732020f8",
    scryfall_id = "64b09c44-d463-45a9-9fa2-89407c21200b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Exiles any card from your graveyard to pay its activation cost; requiring the exiled card to be the top card of your graveyard is not yet modeled."
    ),
    faces = &[face!(
        name = "Nature's Kiss",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        activated!(
            cost!("{1}", ExileFromGraveyard(&Filter::Any)),
            &[Effect::ModifyAttached {
                modifier: Modifier::ModifyPT(1, 1),
                duration: Duration::UntilEndOfTurn
            }]
        ),
    ],
);
