//! Lumra, Bellow of the Woods — {4}{G}{G} — Legendary Creature — Elemental Bear
//! Oracle: Reach, vigilance
//! Oracle: Lumra's power and toughness are each equal to the number of lands you control.
//! Oracle: When Lumra enters, mill four cards. Then return all land cards from your graveyard to the battlefield tapped.
//! Set: BLB #183 — Bloomburrow | Scryfall ID: ae4f3aaf-3960-48cd-b34b-32e4ae5ae088 | Oracle ID: 97a84e9d-bfc4-4ca2-b1e8-908dba56ccdb
// IMPLEMENTED — reach and vigilance; the P/T clause as a per-land +1/+1 modifier
// over the 0/0 body the stub carries; and the enters trigger's mill four. The
// trigger's second sentence is dropped, so the card is `Coverage::Partial`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LUMRA_BELLOW_OF_THE_WOODS,
    oracle_id = "97a84e9d-bfc4-4ca2-b1e8-908dba56ccdb",
    scryfall_id = "ae4f3aaf-3960-48cd-b34b-32e4ae5ae088",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Lumra, Bellow of the Woods",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::BEAR],
        power = Some(0),
        toughness = Some(0),
    ),],
    keywords = KeywordSet::REACH.union(KeywordSet::VIGILANCE),
    coverage = Coverage::Partial(
        "\"Then return all land cards from your graveyard to the battlefield tapped\" is dropped: the \
         only graveyard-to-battlefield effects are single-target (`GraveyardToBattlefield`) or \
         creatures over every graveyard (`AllGraveyardCreaturesToBattlefield`), and nothing the DSL \
         moves arrives tapped. The P/T clause is a layer-7a characteristic-defining ability, which no \
         `Modifier` sets; it is spelled as the layer-7c `Modifier::ModifyPTPerCount`, which is the \
         printed number on the 0/0 body the stub carries and comes out different under a layer-7b \
         effect that sets power and toughness",
    ),
    abilities = &[
        // "Lumra's power and toughness are each equal to the number of lands you control" — one
        // +1/+1 for each land its controller controls, counted off the 0/0 body above.
        static_ability!(
            Filter::This,
            Modifier::ModifyPTPerCount {
                filter: &Filter::YOUR_LAND,
                p: 1,
                t: 1,
            }
        ),
        // "When Lumra enters, mill four cards."
        triggered!(
            Trigger::ETB,
            &[Effect::Mill {
                amount: Amount::Fixed(4),
                target: PlayerRel::You,
            }]
        ),
        // NOT SUPPORTED: "Then return all land cards from your graveyard to the battlefield tapped."
        // No `Effect` moves every card matching a filter out of a graveyard, and no destination the
        // DSL can name enters the battlefield tapped.
    ],
);
