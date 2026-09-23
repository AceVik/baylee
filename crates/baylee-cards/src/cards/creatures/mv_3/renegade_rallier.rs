//! Renegade Rallier — {1}{G}{W} — Creature — Human Warrior
//! Oracle: Revolt — When this creature enters, if a permanent left the battlefield under your control this turn, return target permanent card with mana value 2 or less from your graveyard to the battlefield.
//! Set: AER #133 — Aether Revolt | Scryfall ID: 90bad312-80e3-45b0-9556-60ce06808a47 | Oracle ID: 6fa07b6c-f01a-4416-b0fc-986b0fc4e412
// PARTIAL — the enter trigger and its reanimation are built; revolt's
// intervening-if has no `Condition` variant (NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Target permanent card with mana value 2 or less" — a card that is not an
/// instant or a sorcery, which is what makes a card a permanent card
/// (CR 110.4a). Named because this card refers to it twice, once for the
/// target requirement and once for the effect.
static PERMANENT_CARD_MV_2: Filter = Filter::And(&[
    Filter::CmcAtMost(2),
    Filter::LacksType(TypeSet::INSTANT.union(TypeSet::SORCERY)),
]);

card!(
    index = index::RENEGADE_RALLIER,
    oracle_id = "6fa07b6c-f01a-4416-b0fc-986b0fc4e412",
    scryfall_id = "90bad312-80e3-45b0-9556-60ce06808a47",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Partial(
        "revolt: no Condition variant for \"a permanent left the battlefield under your control this turn\", so the ETB trigger fires unconditionally",
    ),
    faces = &[face!(
        name = "Renegade Rallier",
        mana_cost = mana!("{1}{G}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[
        // NOT SUPPORTED: "if a permanent left the battlefield under your control
        // this turn" — revolt is an intervening-if (CR 603.4), and `Condition`
        // carries no sentence about a permanent having left the battlefield, so
        // the trigger below is unconditional. Everything else on the line is the
        // card: the enter event, the graveyard target, its mana-value cap, and
        // where the card goes.
        triggered!(
            Trigger::ETB,
            &[Effect::reanimate(TargetSpec::CardInGraveyard(
                &PERMANENT_CARD_MV_2,
                PlayerRel::You
            ))],
            targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &PERMANENT_CARD_MV_2,
                PlayerRel::You,
            ))),
        ),
    ],
);
