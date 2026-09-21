//! Vance's Blasting Cannons // Spitfire Bastion — {3}{R} — Legendary Enchantment // Legendary Land
//! Oracle: At the beginning of your upkeep, exile the top card of your library. If it's a nonland card, you may cast that card this turn.
//! Oracle: Whenever you cast your third spell in a turn, you may transform Vance's Blasting Cannons.
//! Oracle: (Transforms from Vance's Blasting Cannons.)
//! Oracle: {T}: Add {R}.
//! Oracle: {2}{R}, {T}: Spitfire Bastion deals 3 damage to any target.
//! Set: XLN #173 — Ixalan | Scryfall ID: 9e8c0009-787f-480b-84b6-bf297f1fb466 | Oracle ID: 5e7eca9c-a7b8-4b7b-a0a0-e8937530145a
//! Face: Vance's Blasting Cannons — {3}{R} — Legendary Enchantment
//! Face: Spitfire Bastion —  — Legendary Land
// PARTIAL — the third-spell transform trigger and both abilities of the back
// face are built; the upkeep trigger cannot be said (see the NOT SUPPORTED
// line below). The back face is the only face a player may not cast, so the
// cast wizard must not offer Bastion as a mode at its unprinted cost.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "At the beginning of your upkeep, exile the top card of your library. If it's a nonland card, you may cast that card this turn." — no effect puts the top card of a library anywhere (nothing like `ExileTopOfLibrary`), and no variant grants a permission to cast the exiled card this turn.

static BACK_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
    activated!(
        cost!("{2}{R}", TapSelf),
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::AnyTarget,
        }],
        target = Some(TargetSpec::AnyTarget)
    ),
];

card!(
    index = index::VANCE_S_BLASTING_CANNONS,
    oracle_id = "5e7eca9c-a7b8-4b7b-a0a0-e8937530145a",
    scryfall_id = "9e8c0009-787f-480b-84b6-bf297f1fb466",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Vance's Blasting Cannons",
            mana_cost = mana!("{3}{R}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Spitfire Bastion",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = BACK_ABILITIES,
            castable_from_hand = false,
        ),
    ],
    coverage = Coverage::Partial(
        "the upkeep trigger: exiling the top card of your library, and casting it this turn, is not expressible"
    ),
    abilities = &[triggered!(
        Trigger::NthSpellCast {
            n: 3,
            filter: &Filter::Any,
        },
        &[Effect::MayDo {
            effects: &[Effect::ExileSelfReturnAsFace { face: 1 }],
        }]
    )],
);
