//! Bone Harvest — {2}{B} — Instant
//! Oracle: Put any number of target creature cards from your graveyard on top of your library.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: MIR #108 — Mirage | Scryfall ID: 1cfde1ca-52f9-477d-a36e-6e4f7ca2e4d8 | Oracle ID: 0cc38d20-5ae8-413a-9f26-ff13cd92f24c
// PARTIAL — puts exactly one target creature card from your graveyard on top of your library, and draws at the next turn's upkeep.
// NOT SUPPORTED: "any number of target creature cards" from your graveyard (including none). The only many-card form, `Effect::TargetedGraveyardToTop`, takes its graveyard from a first target player, and no target spec names only you, so it would let the caster pick any graveyard; the card keeps one target in your own, which is less than printed.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BONE_HARVEST,
    oracle_id = "0cc38d20-5ae8-413a-9f26-ff13cd92f24c",
    scryfall_id = "1cfde1ca-52f9-477d-a36e-6e4f7ca2e4d8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "puts target creature card from your graveyard on top of your library and draws a card at the beginning of the next turn's upkeep; targeting any number of creature cards (including zero) rather than a single target creature card is not supported",
    ),
    faces = &[face!(
        name = "Bone Harvest",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::GraveyardToTop {
                target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))),
    )],
);
