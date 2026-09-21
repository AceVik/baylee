//! Rustic Clachan — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Kithkin card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {W}.
//! Oracle: Reinforce 1—{1}{W} ({1}{W}, Discard this card: Put a +1/+1 counter on target creature.)
//! Set: DDF #34 — Duel Decks: Elspeth vs. Tezzeret | Scryfall ID: f50160e9-0b25-4e81-814d-cdcda2fb325d | Oracle ID: cde68428-0033-4ede-92f1-ab91de0a41fb
// PARTIAL — the mana line and Reinforce 1 are built; the as-it-enters reveal
// is not expressible.
// NOT SUPPORTED: "As this land enters, you may reveal a Kithkin card from
// your hand. If you don't, this land enters tapped." — every `EnterModifier`
// is read against the source's controller and the battlefield, so none of
// them can name a card in a player's hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RUSTIC_CLACHAN,
    oracle_id = "cde68428-0033-4ede-92f1-ab91de0a41fb",
    scryfall_id = "f50160e9-0b25-4e81-814d-cdcda2fb325d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(name = "Rustic Clachan", types = TypeSet::LAND,),],
    coverage = Coverage::Partial("enters tapped unless you reveal a Kithkin card from your hand"),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            cost!("{1}{W}", DiscardSelf),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
            zone = ActivationZone::Hand,
        ),
    ],
);
