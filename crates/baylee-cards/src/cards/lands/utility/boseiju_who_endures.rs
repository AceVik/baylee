//! Boseiju, Who Endures — (no cost) — Legendary Land
//! Oracle: {T}: Add {G}.
//! Oracle: Channel — {1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls. That player may search their library for a land card with a basic land type, put it onto the battlefield, then shuffle. This ability costs {1} less to activate for each legendary creature you control.
//! Set: NEO #266 — Kamigawa: Neon Dynasty | Scryfall ID: 2135ac5a-187b-4dc9-8f82-34e8d1603416 | Oracle ID: bf1341dd-41a3-49f6-87ec-63170dde4324
// PARTIAL — {T}: Add {G}, plus the channel ability as an activation from hand
// that discards this card and destroys an artifact, an enchantment, or a
// nonbasic land an opponent controls. The channel's cost reduction and the
// search its victim makes are not expressible.

use baylee_cards_dsl::prelude::*;

/// "target artifact, enchantment, or nonbasic land an opponent controls" —
/// the controller clause sits on the land, which is where the card prints it.
static DESTROY_TARGETS: Filter = Filter::Or(&[
    Filter::ARTIFACT,
    Filter::ENCHANTMENT,
    f!(opponents NONBASIC_LAND),
]);

card!(
    index = index::BOSEIJU_WHO_ENDURES,
    oracle_id = "bf1341dd-41a3-49f6-87ec-63170dde4324",
    scryfall_id = "2135ac5a-187b-4dc9-8f82-34e8d1603416",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "the channel ability's cost reduction and the searched-for land's \
         untapped, basic-land-type destination are not expressible"
    ),
    faces = &[face!(
        name = "Boseiju, Who Endures",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // NOT SUPPORTED: "This ability costs {1} less to activate for each
        // legendary creature you control." — `CostReduction` carries only
        // `NotStartingPlayer(n)`, and nothing reduces an activation cost per
        // permanent, so this activation always costs its printed {1}{G}.
        // NOT SUPPORTED: "That player may search their library for a land
        // card with a basic land type, put it onto the battlefield, then
        // shuffle." — the nearest variant,
        // `Effect::OptionalBasicLandSearchFor`, hands that player a *basic*
        // land and puts it onto the battlefield *tapped*, which is neither
        // the filter nor the destination this card prints.
        activated!(
            cost!("{1}{G}", DiscardSelf),
            &[Effect::destroy(TargetSpec::Object(&DESTROY_TARGETS))],
            zone = ActivationZone::Hand,
            target = Some(TargetSpec::Object(&DESTROY_TARGETS)),
        ),
    ],
);
