//! Otawara, Soaring City — (no cost) — Legendary Land
//! Oracle: {T}: Add {U}.
//! Oracle: Channel — {3}{U}, Discard this card: Return target artifact, creature, enchantment, or planeswalker to its owner's hand. This ability costs {1} less to activate for each legendary creature you control.
//! Set: NEO #271 — Kamigawa: Neon Dynasty | Scryfall ID: 486d7edc-d983-41f0-8b78-c99aecd72996 | Oracle ID: e9b6a394-691c-425a-9307-76d8edc7375e
// PARTIAL — {T}: Add {U}, plus Channel as a from-hand activated ability
// ({3}{U}, DiscardSelf, bounce); the channel cost reduction is not sayable.

use baylee_cards_dsl::prelude::*;

/// The printed channel target: "artifact, creature, enchantment, or
/// planeswalker". Named because the card refers to it twice — once as the
/// ability's target requirement and once as the bounce's binding.
static CHANNEL_TARGET: Filter = Filter::Or(&[
    Filter::ARTIFACT,
    Filter::CREATURE,
    Filter::ENCHANTMENT,
    Filter::PLANESWALKER,
]);

card!(
    index = index::OTAWARA_SOARING_CITY,
    oracle_id = "e9b6a394-691c-425a-9307-76d8edc7375e",
    scryfall_id = "486d7edc-d983-41f0-8b78-c99aecd72996",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Otawara, Soaring City",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "Channel's \"costs {1} less to activate for each legendary creature you \
         control\" is not expressible: cost.rs's CostReduction carries only \
         NotStartingPlayer(n), with no count-based variant"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        // NOT SUPPORTED: "This ability costs {1} less to activate for each
        // legendary creature you control." — CostReduction has no
        // per-legendary-creature reduction, so the channel ability is
        // activated at its full printed {3}{U}.
        activated!(
            cost!("{3}{U}", DiscardSelf),
            &[Effect::bounce(TargetSpec::Object(&CHANNEL_TARGET))],
            target = Some(TargetSpec::Object(&CHANNEL_TARGET)),
            zone = ActivationZone::Hand,
        ),
    ],
);
