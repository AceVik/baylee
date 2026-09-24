//! Storm the Vault // Vault of Catlacan — {2}{U}{R} — Legendary Enchantment // Legendary Land
//! Oracle: Whenever one or more creatures you control deal combat damage to a player, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
//! Oracle: At the beginning of your end step, if you control five or more artifacts, transform Storm the Vault.
//! Oracle: (Transforms from Storm the Vault.)
//! Oracle: {T}: Add one mana of any color.
//! Oracle: {T}: Add {U} for each artifact you control.
//! Set: RIX #173 — Rivals of Ixalan | Scryfall ID: c16ba84e-a0cc-4c6c-9b80-713247b8fef9 | Oracle ID: 72205fac-a94a-45cc-94c6-40ece2fdce0e
//! Face: Storm the Vault — {2}{U}{R} — Legendary Enchantment
//! Face: Vault of Catlacan —  — Legendary Land
// PARTIAL — the back face's two mana abilities are any color and {U} per
// artifact you control. The combat-damage trigger makes a Treasure per
// creature rather than per batch, and the end-step "transform" is
// exile-and-return (see the NOT SUPPORTED lines beside each).

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
         deals combat damage makes its own Treasure; and no Effect transforms a \
         permanent in place (#206), so ExileSelfReturnAsFace returns Vault of \
         Catlacan as a new object that enters"
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
            // NOT SUPPORTED: "transform Storm the Vault" as printed. To
            // transform is to turn the permanent over (CR 701.27a), and it
            // stays the same object (CR 712.18). `ExileSelfReturnAsFace`
            // exiles it and returns a new object, so Vault of Catlacan
            // *enters*: landfall and "whenever a land enters" see it, and
            // anything that applied to the enchantment is gone. #206.
            &[Effect::ExileSelfReturnAsFace { face: 1 }],
            condition = Some(Condition::ControlCount(&Filter::ARTIFACT, 5)),
        ),
    ],
);
