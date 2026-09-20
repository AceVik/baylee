//! Hive of the Eye Tyrant — (no cost) — Land
//! Oracle: If you control two or more other lands, this land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {3}{B}: Until end of turn, this land becomes a 3/3 black Beholder creature with menace and "Whenever this creature attacks, exile target card from defending player's graveyard." It's still a land.
//! Set: AFR #258 — Adventures in the Forgotten Realms | Scryfall ID: 9eb391dc-0378-4793-a5de-899b09792a4b | Oracle ID: d17163d4-dd43-4de6-b7cf-576448160b7f
// PARTIAL — the entry condition, the {B} mana ability and the whole animation
// are built: the card turns into a 3/3 black Beholder creature that is still a
// land, with menace and the granted attack trigger. One printed phrase has no
// variant — see the `// NOT SUPPORTED:` line at the trigger.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

// NOT SUPPORTED: "exile target card from the defending player's graveyard" — no PlayerRel names the defending player, so PlayerRel::Opponent stands in: exact in a two-player game, approximate in multiplayer (M3).
// "Whenever this creature attacks, exile target card from defending player's
// graveyard." — the ability the animation grants, and the one target it asks
// for.
static EXILE_DEFENDING_GRAVEYARD: &[Effect] = &[Effect::Exile {
    target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::Opponent),
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
                PlayerRel::Opponent,
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
    coverage = Coverage::Partial(
        "the granted attack trigger targets a card in the defending player's graveyard; PlayerRel has no defending player, so PlayerRel::Opponent is read — exact in a two-player game, approximate in multiplayer"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(cost!("{3}{B}"), ANIMATE),
    ],
);
