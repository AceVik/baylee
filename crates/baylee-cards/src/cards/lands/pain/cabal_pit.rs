//! Cabal Pit — (no cost) — Land
//! Oracle: {T}: Add {B}. This land deals 1 damage to you.
//! Oracle: Threshold — {B}, {T}, Sacrifice this land: Target creature gets -2/-2 until end of turn. Activate only if there are seven or more cards in your graveyard.
//! Set: ODY #315 — Odyssey | Scryfall ID: 848d686a-e2f7-488d-947f-a555099b74b1 | Oracle ID: 92392467-a22f-4dd7-a0eb-393bef956dc0
// PARTIAL — the mana line is exact: one mana ability that adds {B} and
// deals the point of damage to its own controller. The Threshold ability is
// built, but has no activation condition the DSL can state.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CABAL_PIT,
    oracle_id = "92392467-a22f-4dd7-a0eb-393bef956dc0",
    scryfall_id = "848d686a-e2f7-488d-947f-a555099b74b1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(name = "Cabal Pit", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "Threshold — \"activate only if there are seven or more cards in your graveyard\": no Condition variant counts cards in a graveyard"
    ),
    abilities = &[
        // "{T}: Add {B}. This land deals 1 damage to you." One ability and
        // not two: the damage is part of the mana line, and CR 605.1a still
        // makes it a mana ability (it adds mana, targets nothing, and is not
        // a loyalty ability). The damage is `DealDamage` and not `LoseLife`
        // — the card prints "deals", so prevention and every "whenever you're
        // dealt damage" trigger has its say.
        mana_ability!(&[
            Effect::mana(ManaColor::Black, 1),
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            },
        ]),
        // NOT SUPPORTED: "Activate only if there are seven or more cards in
        // your graveyard." `Condition` answers `ControlCount`,
        // `OpponentGraveyardCountAtLeast`, `CountersOnSelf`,
        // `CountersOnSelfExactly` and `SourceMatches`; none of them asks
        // about the cards in *your* graveyard (`Amount::CountOf` can, but an
        // amount is not a condition). So the ability below is offered with
        // no threshold at all.
        activated!(
            cost!("{B}", TapSelf, SacrificeSelf),
            &[Effect::PumpTarget {
                power: Amount::NegXFixed(2),
                toughness: Amount::NegXFixed(2),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
