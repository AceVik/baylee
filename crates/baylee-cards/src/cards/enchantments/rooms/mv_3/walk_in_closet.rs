//! Walk-In Closet // Forgotten Cellar — {2}{G} — Enchantment — Room // Enchantment — Room
//! Oracle: You may play lands from your graveyard.
//! Oracle: (You may cast either half. That door unlocks on the battlefield. As a sorcery, you may pay the mana cost of a locked door to unlock it.)
//! Oracle: When you unlock this door, you may cast spells from your graveyard this turn, and if a card would be put into your graveyard from anywhere this turn, exile it instead.
//! Oracle: (You may cast either half. That door unlocks on the battlefield. As a sorcery, you may pay the mana cost of a locked door to unlock it.)
//! Set: DSK #205 — Duskmourn: House of Horror | Scryfall ID: 0adcd4e5-d542-4293-8774-ace2305ef820 | Oracle ID: 52e77cc3-f8e9-4a20-811b-fe1e46a96ad7
//! Face: Walk-In Closet — {2}{G} — Enchantment — Room
//! Face: Forgotten Cellar — {3}{G}{G} — Enchantment — Room
// PARTIAL — the Room is whole: cast either half and that door enters
// unlocked; the other is unlocked as a sorcery for its mana cost; a locked
// door has no rules text. Walk-In Closet plays lands from the graveyard, and
// Forgotten Cellar's unlock trigger exiles what would reach the graveyard
// this turn. Its permission to cast spells from the graveyard this turn is
// the one clause missing: that is the library group's graveyard-cast
// machinery (`casting::graveyard_cast_permission` on c42/cards-library),
// which casts permanent spells only, and is not on this branch.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALK_IN_CLOSET,
    oracle_id = "52e77cc3-f8e9-4a20-811b-fe1e46a96ad7",
    scryfall_id = "0adcd4e5-d542-4293-8774-ace2305ef820",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Walk-In Closet",
            mana_cost = mana!("{2}{G}"),
            types = TypeSet::ENCHANTMENT,
            subtypes = &[subtypes::enchantment::ROOM],
            abilities = &[static_ability!(
                Filter::Any,
                Modifier::PlayLandsFromGraveyard
            )],
        ),
        face!(
            name = "Forgotten Cellar",
            mana_cost = mana!("{3}{G}{G}"),
            types = TypeSet::ENCHANTMENT,
            subtypes = &[subtypes::enchantment::ROOM],
            // NOT SUPPORTED: "you may cast spells from your graveyard this turn" — no permission casts every spell from a graveyard for a turn; the library group's `CastPermanentSpellsFromGraveyard` casts permanent spells only.
            abilities = &[triggered!(
                Trigger::UnlockThisDoor(1),
                &[Effect::continuous(
                    &Filter::Any,
                    Modifier::ExileInsteadOfYourGraveyard,
                    Duration::UntilEndOfTurn
                )]
            )],
        ),
    ],
    coverage = Coverage::Partial(
        "Forgotten Cellar's \"you may cast spells from your graveyard this turn\": nothing grants casting every spell from a graveyard for a turn (the library group's graveyard-cast permission casts permanent spells only)"
    ),
    // Both doors unlocked (CR 709.5): both halves' rules text, the left
    // half's first.
    abilities = &[
        static_ability!(Filter::Any, Modifier::PlayLandsFromGraveyard),
        triggered!(
            Trigger::UnlockThisDoor(1),
            &[Effect::continuous(
                &Filter::Any,
                Modifier::ExileInsteadOfYourGraveyard,
                Duration::UntilEndOfTurn
            )]
        ),
    ],
);
