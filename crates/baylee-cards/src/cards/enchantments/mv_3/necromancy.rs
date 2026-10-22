//! Necromancy — {2}{B} — Enchantment
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: When this enchantment enters, if it's on the battlefield, it becomes an Aura with "enchant creature put onto the battlefield with Necromancy." Put target creature card from a graveyard onto the battlefield under your control and attach this enchantment to it. When this enchantment leaves the battlefield, that creature's controller sacrifices it.
//! Set: MKC #131 — Murders at Karlov Manor Commander | Scryfall ID: 3dd34a1a-e76b-46ee-be8d-5f7f4a092648 | Oracle ID: 699b192c-f3f2-4ac5-9ca9-e35bcf08ad8a
// PARTIAL: has flash and returns target creature card from a graveyard to the battlefield under your control when it enters; sacrificing Necromancy at the next cleanup step if cast at instant speed, becoming an Aura attached to the returned creature, and sacrificing that creature when Necromancy leaves the battlefield are not yet supported.

use baylee_cards_dsl::prelude::*;

const GRAVEYARD_CREATURE: TargetSpec =
    TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::EachPlayer);

// NOT SUPPORTED: cleanup-step sacrifice when cast at instant speed; becoming an Aura attached to the returned creature and sacrificing that creature when Necromancy leaves the battlefield.
card!(
    index = index::NECROMANCY,
    oracle_id = "699b192c-f3f2-4ac5-9ca9-e35bcf08ad8a",
    scryfall_id = "3dd34a1a-e76b-46ee-be8d-5f7f4a092648",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Partial(
        "has flash and returns target creature card from a graveyard to the battlefield under your control when it enters; sacrificing Necromancy at the next cleanup step if cast at instant speed, becoming an Aura attached to the returned creature, and sacrificing that creature when Necromancy leaves the battlefield are not yet supported"
    ),
    faces = &[face!(
        name = "Necromancy",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::GraveyardToBattlefield {
            target: GRAVEYARD_CREATURE,
            owner_control: false,
            counters: None,
        }],
        targets = Some(TargetReq::one(GRAVEYARD_CREATURE)),
    )],
);
