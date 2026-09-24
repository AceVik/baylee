//! Grasping Shadows // Shadows' Lair — {3}{B} — Enchantment // Land — Cave
//! Oracle: Whenever a creature you control attacks alone, it gains deathtouch and lifelink until end of turn. Put a dread counter on this enchantment. Then if there are three or more dread counters on it, transform it.
//! Oracle: (Transforms from Grasping Shadows.)
//! Oracle: {T}: Add {B}.
//! Oracle: {B}, {T}, Remove a dread counter from this land: You draw a card and you lose 1 life.
//! Set: LCI #108 — The Lost Caverns of Ixalan | Scryfall ID: 81b8b9c9-725d-476d-a3cf-55e3dc3e433d | Oracle ID: 522a4b02-24c7-45d2-9097-2803cc9fffad
//! Face: Grasping Shadows — {3}{B} — Enchantment
//! Face: Shadows' Lair —  — Land — Cave
// PARTIAL — the lone-attacker trigger is built up to its last sentence, and
// Shadows' Lair is whole; what is missing is the transform between them, so
// the enchantment gathers dread counters and never turns over.

// NOT SUPPORTED: "Then if there are three or more dread counters on it,
// transform it." — the branch on three is sayable (Effect::IfCondition over
// Condition::CountersOnSelf(counters::DREAD, 3)); the transform it leads to
// is not: no effect turns a permanent over in place (#206). The
// exile-and-return Thaumatic Compass writes instead
// (Effect::ExileSelfReturnAsFace) would bring back a new object without the
// dread counters Shadows' Lair spends.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Shadows' Lair's abilities — the back face of a transforming card, so they
/// belong to that face and not to the enchantment the card is played as.
static BACK_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
    activated!(
        cost!(
            "{B}",
            TapSelf,
            RemoveCounterSelf {
                kind: counters::DREAD,
                n: 1
            }
        ),
        &[
            Effect::draw(1),
            Effect::LoseLife {
                amount: Amount::Fixed(1),
                target: PlayerRel::You,
            },
        ]
    ),
];

card!(
    index = index::GRASPING_SHADOWS,
    oracle_id = "522a4b02-24c7-45d2-9097-2803cc9fffad",
    scryfall_id = "81b8b9c9-725d-476d-a3cf-55e3dc3e433d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Grasping Shadows",
            mana_cost = mana!("{3}{B}"),
            types = TypeSet::ENCHANTMENT,
        ),
        face!(
            name = "Shadows' Lair",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::CAVE],
            abilities = BACK_ABILITIES,
        ),
    ],
    coverage = Coverage::Partial(
        "no effect transforms a permanent in place (#206), so the dread counters gather on the enchantment and Shadows' Lair is never reached",
    ),
    // "Attacks alone" is the only creature *declared* as an attacker
    // (CR 506.5), and the only spelling for it is an intervening `if` on the
    // attack trigger: you control at most one attacking creature. CR 603.4
    // asks that again on resolution, where the printed sentence does not —
    // but in this engine a creature becomes attacking only by being declared
    // (`declare_attackers` is the one writer of `combat.attackers`; everything
    // else removes), so the count can only fall between the two checks and
    // the second one cannot fail where the first held. The day something is
    // put onto the battlefield attacking (CR 508.4), this spelling is wrong
    // and "attacks alone" needs a trigger of its own.
    //
    // "It" is the attacker (the event object, CR 115.10a: no target is
    // chosen), which `Filter::This` resolves to inside the continuous
    // effect; the dread counter goes on the enchantment, which is what
    // `AddCounterFilter` over `Filter::This` finds (`AddCounter` would follow
    // the event object onto the creature).
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::YOUR_CREATURE),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::DEATHTOUCH.union(KeywordSet::LIFELINK)),
                Duration::UntilEndOfTurn
            ),
            Effect::AddCounterFilter {
                filter: &Filter::This,
                kind: counters::DREAD,
                amount: Amount::Fixed(1),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::EventObject)),
        condition = Some(Condition::ControlCountAtMost(
            &Filter::ATTACKING_CREATURE,
            1
        )),
    )],
);
