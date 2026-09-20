//! Vault of the Archangel — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}{W}{B}, {T}: Creatures you control gain deathtouch and lifelink until end of turn.
//! Set: TDC #410 — Tarkir: Dragonstorm Commander | Scryfall ID: 8f203f04-bd73-4d88-bb5e-aab63507dbd1 | Oracle ID: eeaac65a-3480-475a-bb28-e6375d53f487
// IMPLEMENTED — {T}: Add {C}, plus the team buff: one `PumpFilter` on
// `Filter::YOUR_CREATURE` with a +0/+0 pump carrying both keywords until end
// of turn. Nothing target is chosen, so no `TargetReq` is written (CR 115.1c).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VAULT_OF_THE_ARCHANGEL,
    oracle_id = "eeaac65a-3480-475a-bb28-e6375d53f487",
    scryfall_id = "8f203f04-bd73-4d88-bb5e-aab63507dbd1",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(
        name = "Vault of the Archangel",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}{W}{B}", TapSelf),
            &[Effect::PumpFilter {
                filter: &Filter::YOUR_CREATURE,
                controlled_by: None,
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::DEATHTOUCH.union(KeywordSet::LIFELINK),
                duration: Duration::UntilEndOfTurn,
            }],
        ),
    ],
);
