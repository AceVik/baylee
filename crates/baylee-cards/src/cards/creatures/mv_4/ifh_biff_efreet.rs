//! Ifh-Bíff Efreet — {2}{G}{G} — Creature — Efreet
//! Oracle: Flying
//! Oracle: {G}: This creature deals 1 damage to each creature with flying and each player. Any player may activate this ability.
//! Set: ME1 #123 — Masters Edition | Scryfall ID: f4c21c0d-91ee-4c2c-bfa4-81bb07106842 | Oracle ID: e503a4f2-a785-4e7a-89a7-a9b24fb98831
// PARTIAL — flying and the {G} damage are written; the ability is offered
// only to this creature's controller.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IFH_BIFF_EFREET,
    oracle_id = "e503a4f2-a785-4e7a-89a7-a9b24fb98831",
    scryfall_id = "f4c21c0d-91ee-4c2c-bfa4-81bb07106842",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Ifh-Bíff Efreet",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::EFREET],
        power = Some(3),
        toughness = Some(3),
        keywords = KeywordSet::FLYING,
    ),],
    coverage = Coverage::Partial(
        "no activation permission offers an ability to any player but its \
         controller, so the {G} ability is controller-only"
    ),
    // NOT SUPPORTED: "Any player may activate this ability." — the
    // activation offer is the controller's alone (CR 602.2) and no
    // `AbilityDef` field names another activator. `TargetSpec::AnyPlayer`
    // and `PlayerRel::EachPlayer` choose players a resolution acts on, and
    // neither puts this ability in another player's legal actions. The
    // damage itself is written below and is offered only to the controller.
    abilities = &[activated!(
        cost!("{G}"),
        &[
            Effect::DealDamageEach {
                amount: Amount::Fixed(1),
                filter: &Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING),]),
            },
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::EachPlayer),
            },
        ]
    )],
);
