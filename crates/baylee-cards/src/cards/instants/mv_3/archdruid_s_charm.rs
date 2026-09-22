//! Archdruid's Charm — {G}{G}{G} — Instant
//! Oracle: Choose one —
//! Oracle: • Search your library for a creature or land card and reveal it. Put it onto the battlefield tapped if it's a land card. Otherwise, put it into your hand. Then shuffle.
//! Oracle: • Put a +1/+1 counter on target creature you control. It deals damage equal to its power to target creature you don't control.
//! Oracle: • Exile target artifact or enchantment.
//! Set: MKM #151 — Murders at Karlov Manor | Scryfall ID: 5caae5ae-845f-42c2-b1ae-956df2739433 | Oracle ID: 3c1ef404-e2c6-486d-a5a2-d5779c71d498
// PARTIAL — mode 3 is built. Modes 1 and 2 are dropped with a
// `// NOT SUPPORTED:` line each: neither has a spelling in the DSL. With one
// mode left there is no choice to offer, so the remainder is written as the
// plain spell it now is rather than as a one-option `ModalSpell`.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "Search your library for a creature or land card and reveal
// it. Put it onto the battlefield tapped if it's a land card. Otherwise, put
// it into your hand. Then shuffle." — `Effect::SearchLibrary`'s `finds` pair
// up with the found cards *positionally* and say nothing about what each card
// is, so one search whose destination forks on the found card's type (land →
// battlefield tapped, creature → hand) has no spelling here. Two searches
// would find two cards, and `Find` carries no condition.
// NOT SUPPORTED: "Put a +1/+1 counter on target creature you control. It
// deals damage equal to its power to target creature you don't control." —
// a `TargetReq` carries one `spec`, so a mode requiring one target matching
// `YOUR_CREATURE` *and* one matching "a creature you don't control" is not
// sayable; and `Effect::DealDamage` takes a `TargetSpec`, with no way to aim
// at the *second* chosen target nor to read that target's power as the amount.

card!(
    index = index::ARCHDRUID_S_CHARM,
    oracle_id = "3c1ef404-e2c6-486d-a5a2-d5779c71d498",
    scryfall_id = "5caae5ae-845f-42c2-b1ae-956df2739433",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Archdruid's Charm",
        mana_cost = mana!("{G}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "two of the three modes are not expressible: no search destination \
         that forks on the found card's type (mode 1), and no mode that \
         targets one creature you control plus one you don't, nor an amount \
         read off the second target's power (mode 2)"
    ),
    abilities = &[spell!(
        &[Effect::exile(TargetSpec::Object(
            &Filter::ARTIFACT_OR_ENCHANTMENT
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ARTIFACT_OR_ENCHANTMENT
        ))),
    )],
);
