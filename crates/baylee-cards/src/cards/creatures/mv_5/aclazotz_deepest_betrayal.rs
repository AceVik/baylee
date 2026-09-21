//! Aclazotz, Deepest Betrayal // Temple of the Dead — {3}{B}{B} — Legendary Creature — Bat God // Land
//! Oracle: Flying, lifelink
//! Oracle: Whenever Aclazotz attacks, each opponent discards a card. For each opponent who can't, you draw a card.
//! Oracle: Whenever an opponent discards a land card, create a 1/1 black Bat creature token with flying.
//! Oracle: When Aclazotz dies, return it to the battlefield tapped and transformed under its owner's control.
//! Oracle: (Transforms from Aclazotz, Deepest Betrayal.)
//! Oracle: {T}: Add {B}.
//! Oracle: {2}{B}, {T}: Transform this land. Activate only if a player has one or fewer cards in hand and only as a sorcery.
//! Set: LCI #88 — The Lost Caverns of Ixalan | Scryfall ID: 627c392c-4d18-4eb2-a4e8-c668f61f5487 | Oracle ID: fcdfe9d5-2743-4d3e-ab57-bf0f96beaa15
//! Face: Aclazotz, Deepest Betrayal — {3}{B}{B} — Legendary Creature — Bat God
//! Face: Temple of the Dead —  — Land
// PARTIAL — flying and lifelink, the attack trigger's discard, and the back
// face's {T}: Add {B}. The transform mechanic, the opponent-discards trigger
// and the dies return have no shape in the DSL yet.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Temple of the Dead's one expressible ability.
static BACK_FACE_ABILITIES: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])];

card!(
    index = index::ACLAZOTZ_DEEPEST_BETRAYAL,
    oracle_id = "fcdfe9d5-2743-4d3e-ab57-bf0f96beaa15",
    scryfall_id = "627c392c-4d18-4eb2-a4e8-c668f61f5487",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING.union(KeywordSet::LIFELINK),
    coverage = Coverage::Partial(
        "no Effect::Transform, no trigger for an opponent's discard, and no way to \
         return a card from a graveyard to the battlefield transformed and tapped",
    ),
    faces = &[
        face!(
            name = "Aclazotz, Deepest Betrayal",
            mana_cost = mana!("{3}{B}{B}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::BAT, subtypes::creature::GOD],
            power = Some(4),
            toughness = Some(4),
        ),
        face!(
            name = "Temple of the Dead",
            types = TypeSet::LAND,
            abilities = BACK_FACE_ABILITIES,
        ),
    ],
    abilities = &[
        // NOT SUPPORTED: "For each opponent who can't, you draw a card." — no
        // `Amount` counts the opponents who could not discard.
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::DiscardForPlayers {
                who: PlayerRel::EachOpponent,
                count: 1,
            }]
        ),
        // NOT SUPPORTED: "Whenever an opponent discards a land card, create a
        // 1/1 black Bat creature token with flying." — `Trigger` has no
        // discard event to listen for.
        // NOT SUPPORTED: "When Aclazotz dies, return it to the battlefield
        // tapped and transformed under its owner's control." — nothing moves
        // a card out of a graveyard transformed, and nothing returns it tapped.
    ],
);

// NOT SUPPORTED: "{2}{B}, {T}: Transform this land. Activate only if a player
// has one or fewer cards in hand and only as a sorcery." — `Effect` has no
// Transform, and no `Condition` reads a player's hand size.
