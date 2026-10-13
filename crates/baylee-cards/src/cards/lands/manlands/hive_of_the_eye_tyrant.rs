//! Hive of the Eye Tyrant — (no cost) — Land
//! Oracle: If you control two or more other lands, this land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {3}{B}: Until end of turn, this land becomes a 3/3 black Beholder creature with menace and "Whenever this creature attacks, exile target card from defending player's graveyard." It's still a land.
//! Set: AFR #258 — Adventures in the Forgotten Realms | Scryfall ID: 9eb391dc-0378-4793-a5de-899b09792a4b | Oracle ID: d17163d4-dd43-4de6-b7cf-576448160b7f
// IMPLEMENTED — enters tapped with two or more other lands, {T}: Add {B}, and
// {3}{B} animates it into a 3/3 black Beholder creature with menace and the
// attack trigger that exiles a card from the defending player's graveyard.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

// "Whenever this creature attacks, exile target card from defending player's
// graveyard." — the ability the animation grants, and the one target it asks
// for.
static EXILE_DEFENDING_GRAVEYARD: &[Effect] = &[Effect::Exile {
    target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::DefendingPlayer),
}];

// "{3}{B}: Until end of turn, this land becomes a 3/3 black Beholder creature
// with menace and that ability. It's still a land."
//
// One created effect per characteristic, all pointed at the source (the
// ability targets nothing, so `Filter::This` is the land itself) and all until
// end of turn: the creature type and the Beholder subtype in layer 4, black in
// layer 5, menace and the granted trigger in layer 6, 3/3 in layer 7b. "It's
// still a land" is the absence of a RemoveType — the card adds the creature
// type and takes nothing away.
static ANIMATE: &[Effect] = &[
    Effect::continuous(
        &Filter::This,
        Modifier::AddType(TypeSet::CREATURE),
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &Filter::This,
        Modifier::AddSubtype(creature::BEHOLDER),
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &Filter::This,
        Modifier::AddColor(ColorSet::from_slice(&[Color::Black])),
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &Filter::This,
        Modifier::SetPT(3, 3),
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &Filter::This,
        Modifier::AddKeyword(KeywordSet::MENACE),
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &Filter::This,
        Modifier::GrantTriggered {
            trigger: Trigger::Attacks(&Filter::This),
            effects: EXILE_DEFENDING_GRAVEYARD,
            target: Some(TargetSpec::CardInGraveyard(
                &Filter::Any,
                PlayerRel::DefendingPlayer,
            )),
        },
        Duration::UntilEndOfTurn,
    ),
];

card!(
    index = index::HIVE_OF_THE_EYE_TYRANT,
    oracle_id = "d17163d4-dd43-4de6-b7cf-576448160b7f",
    scryfall_id = "9eb391dc-0378-4793-a5de-899b09792a4b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Hive of the Eye Tyrant",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessAtMost {
            filter: &Filter::YOUR_LAND,
            at_most: 1,
        }],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(cost!("{3}{B}"), ANIMATE),
    ],
);
