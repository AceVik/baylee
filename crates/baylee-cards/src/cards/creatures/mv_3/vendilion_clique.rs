//! Vendilion Clique — {1}{U}{U} — Legendary Creature — Faerie Wizard
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: When Vendilion Clique enters, look at target player's hand. You may choose a nonland card from it. If you do, that player reveals the chosen card, puts it on the bottom of their library, then draws a card.
//! Set: SLD #110 — Secret Lair Drop | Scryfall ID: cd702cf1-10ca-4448-9fb1-b6de635e839c | Oracle ID: 244d4807-0802-41bc-9460-55ac38a28a72
// IMPLEMENTED — flash/flying + hand-attack (choose a nonland card from
// the target player's hand, bottom it, draw). The hand reveal is a
// protocol presentation item; the choice itself is engine-complete.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 181,
    oracle_id: "244d4807-0802-41bc-9460-55ac38a28a72",
    scryfall_id: "cd702cf1-10ca-4448-9fb1-b6de635e839c",
    faces: &[face! {
        name: "Vendilion Clique",
        mana_cost: baylee_core::mana!("{1}{U}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::FAERIE, creature::WIZARD],
        power: Some(3),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&Filter::This), &[
            Effect::BottomCardFromHand {
                player: PlayerRel::Opponent,
                filter: &Filter::NONLAND,
            },
            Effect::DrawCardsFor {
                amount: Amount::Fixed(1),
                who: PlayerRel::Opponent,
            },
        ])],
}
