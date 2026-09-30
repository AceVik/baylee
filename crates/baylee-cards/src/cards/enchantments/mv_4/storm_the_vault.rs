//! Storm the Vault // Vault of Catlacan — {2}{U}{R} — Legendary Enchantment // Legendary Land
//! Oracle: Whenever one or more creatures you control deal combat damage to a player, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
//! Oracle: At the beginning of your end step, if you control five or more artifacts, transform Storm the Vault.
//! Oracle: (Transforms from Storm the Vault.)
//! Oracle: {T}: Add one mana of any color.
//! Oracle: {T}: Add {U} for each artifact you control.
//! Set: RIX #173 — Rivals of Ixalan | Scryfall ID: c16ba84e-a0cc-4c6c-9b80-713247b8fef9 | Oracle ID: 72205fac-a94a-45cc-94c6-40ece2fdce0e
//! Face: Storm the Vault — {2}{U}{R} — Legendary Enchantment
//! Face: Vault of Catlacan —  — Legendary Land
// PARTIAL — the end-step transform and the back face's two mana abilities
// (any color, and {U} per artifact you control) are built. The combat-damage
// trigger makes a Treasure per creature rather than per batch (see the NOT
// SUPPORTED line beside it).

use crate::tokens::TREASURE;
use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana_of_any_color()]),
    mana_ability!(&[Effect::mana_dynamic(
        ManaColor::Blue,
        Amount::CountOf {
            filter: &Filter::YOUR_ARTIFACT,
            zone: ZoneSel::Battlefield,
        },
    )]),
];

card!(
    index = index::STORM_THE_VAULT,
    oracle_id = "72205fac-a94a-45cc-94c6-40ece2fdce0e",
    scryfall_id = "c16ba84e-a0cc-4c6c-9b80-713247b8fef9",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[
        face!(
            name = "Storm the Vault",
            mana_cost = mana!("{2}{U}{R}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Vault of Catlacan",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            castable_from_hand = false,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "no Trigger fires once for \"one or more creatures\", so each creature that \
         deals combat damage makes its own Treasure"
    ),
    abilities = &[
        triggered!(
            // NOT SUPPORTED as printed: "Whenever one or more creatures you
            // control deal combat damage to a player" fires once for the
            // whole batch. `DealsCombatDamageToPlayer` fires once per
            // creature (`trigger.rs`, the "No `break`" note), so two
            // connecting creatures make two Treasures.
            Trigger::DealsCombatDamageToPlayer(&Filter::YOUR_CREATURE),
            &[Effect::CreateToken { token: &TREASURE }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You,
            },
            // To transform is to turn the permanent over (CR 701.27a), and
            // it stays the same object (CR 712.18): Vault of Catlacan does
            // not enter, so nothing that watches a land enter sees it.
            &[Effect::TransformSource],
            condition = Some(Condition::ControlCount(&Filter::ARTIFACT, 5)),
        ),
    ],
);
