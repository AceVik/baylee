//! Heart of Yavimaya — (no cost) — Land
//! Oracle: If this land would enter, sacrifice a Forest instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.
//! Oracle: {T}: Add {G}.
//! Oracle: {T}: Target creature gets +1/+1 until end of turn.
//! Set: ME2 #231 — Masters Edition II | Scryfall ID: c05b097f-5b76-49b7-88fb-aef56431df39 | Oracle ID: 6c9a854c-0509-4ed4-9d94-c45b823b65e5
// PARTIAL — {T}: Add {G}, and {T}: target creature gets +1/+1 until end of
// turn; the printed entry replacement is not expressible (see below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HEART_OF_YAVIMAYA,
    oracle_id = "6c9a854c-0509-4ed4-9d94-c45b823b65e5",
    scryfall_id = "c05b097f-5b76-49b7-88fb-aef56431df39",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "the \"If this land would enter, sacrifice a Forest instead. If you do, put \
         this land onto the battlefield. If you don't, put it into its owner's \
         graveyard.\" replacement cannot be expressed"
    ),
    faces = &[face!(name = "Heart of Yavimaya", types = TypeSet::LAND,)],
    // NOT SUPPORTED: "If this land would enter, sacrifice a Forest instead. If you
    // do, put this land onto the battlefield. If you don't, put it into its owner's
    // graveyard." — no EnterModifier states a cost that can keep the land off the
    // battlefield (`TappedOrPayLife` only says how an arriving land arrives), and no
    // ReplacementRule replaces an arrival at all. Written as a `Trigger::ETB` +
    // `PlayerMayPayCostOr` approximation the land would be on the battlefield first,
    // so its own and everyone else's enter-triggers would fire where the printed
    // replacement never lets them. The card is therefore playable without the clause.
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        activated!(
            Cost::TAP,
            &[Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE))
        ),
    ],
);
