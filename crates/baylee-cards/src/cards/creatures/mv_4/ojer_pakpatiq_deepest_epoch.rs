//! Ojer Pakpatiq, Deepest Epoch // Temple of Cyclical Time — {2}{U}{U} — Legendary Creature — God // Land
//! Oracle: Flying
//! Oracle: Whenever you cast an instant spell from your hand, it gains rebound. (Exile it as it resolves. At the beginning of your next upkeep, you may cast it from exile without paying its mana cost.)
//! Oracle: When Ojer Pakpatiq dies, return it to the battlefield tapped and transformed under its owner's control with three time counters on it.
//! Oracle: (Transforms from Ojer Pakpatiq, Deepest Epoch.)
//! Oracle: {T}: Add {U}. Remove a time counter from this land.
//! Oracle: {2}{U}, {T}: Transform this land. Activate only if it has no time counters on it and only as a sorcery.
//! Set: LCI #67 — The Lost Caverns of Ixalan | Scryfall ID: a9d71007-bc04-4dff-ad3f-e2c0b5b4400e | Oracle ID: 34ef174e-1b3d-43d5-9f72-3d35befbdd7f
//! Face: Ojer Pakpatiq, Deepest Epoch — {2}{U}{U} — Legendary Creature — God
//! Face: Temple of Cyclical Time —  — Land
// PARTIAL — flying, the death return and the land's transform are built; the
// rebound grant and the counter the land's mana ability removes are not
// sayable at all.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Temple of Cyclical Time's two abilities.
static TEMPLE_ABILITIES: &[AbilityDef] = &[
    // NOT SUPPORTED: "Remove a time counter from this land" — no `Effect`
    // takes a counter off a permanent. The only two variants that do are the
    // *cost* parts `RemoveCounterSelf` and `RemoveCounterSelfX`, and a cost
    // is paid as the ability is activated (CR 118.3) where the card pays it
    // as the ability resolves; writing it as a cost would also make the land
    // unusable at nought counters, which the printed land is not. Nothing
    // puts a time counter here either (see the death trigger below), so the
    // dropped half has nothing to remove in any state this pool reaches.
    mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
    // Transform, spelled the one way the pool reaches the other face.
    activated!(
        cost!("{2}{U}", TapSelf),
        &[Effect::ExileSelfReturnAsFace { face: 0 }],
        timing = ActivationTiming::SorcerySpeed,
        condition = Some(Condition::CountersOnSelfExactly(CounterKind::Time, 0)),
    ),
];

card!(
    index = index::OJER_PAKPATIQ_DEEPEST_EPOCH,
    oracle_id = "34ef174e-1b3d-43d5-9f72-3d35befbdd7f",
    scryfall_id = "a9d71007-bc04-4dff-ad3f-e2c0b5b4400e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[
        face!(
            name = "Ojer Pakpatiq, Deepest Epoch",
            mana_cost = mana!("{2}{U}{U}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::GOD],
            power = Some(4),
            toughness = Some(3),
            keywords = KeywordSet::FLYING,
        ),
        face!(
            name = "Temple of Cyclical Time",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            abilities = TEMPLE_ABILITIES,
        ),
    ],
    coverage = Coverage::Partial(
        "rebound has no grant-to-a-spell effect, and no effect removes a counter from a land",
    ),
    abilities = &[
        // NOT SUPPORTED: "Whenever you cast an instant spell from your hand,
        // it gains rebound." — no effect hands a keyword to a spell on the
        // stack (`GrantFlashback` is the pool's only grant and it reaches a
        // card in a graveyard), and pointing the trigger at the spell to get
        // one would turn a sentence that does not target into one that does.
        //
        // NOT SUPPORTED: "…tapped and transformed … with three time counters
        // on it" — the return is `ExileSelfReturnAsFace`, but two words of
        // the clause are as-it-enters modifiers (`EnterModifier::Tapped` and
        // `EnterModifier::WithCounters { kind: CounterKind::Time, amount: 3
        // }`), which are carried by a face and by no effect.
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ExileSelfReturnAsFace { face: 1 }]
        ),
    ],
);
