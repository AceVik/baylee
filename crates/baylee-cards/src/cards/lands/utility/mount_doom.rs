//! Mount Doom — (no cost) — Legendary Land
//! Oracle: {T}, Pay 1 life: Add {B} or {R}.
//! Oracle: {1}{B}{R}, {T}: Mount Doom deals 1 damage to each opponent.
//! Oracle: {5}{B}{R}, {T}, Sacrifice Mount Doom and a legendary artifact: Choose up to two creatures, then destroy the rest. Activate only as a sorcery.
//! Set: LTR #258 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: b5bc71a1-2344-4bc6-aa60-658cec19d0d6 | Oracle ID: 995c8dac-fd27-468a-abd4-02372cf0c850
// PARTIAL — the {T}, Pay 1 life mana ability and the {1}{B}{R} ping to each
// opponent are built; the {5}{B}{R} ability is not expressible (see the
// NOT SUPPORTED line below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOUNT_DOOM,
    oracle_id = "995c8dac-fd27-468a-abd4-02372cf0c850",
    scryfall_id = "b5bc71a1-2344-4bc6-aa60-658cec19d0d6",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Partial(
        "no effect chooses a subset of creatures and destroys the rest, \
         which is what the {5}{B}{R} ability does",
    ),
    faces = &[face!(
        name = "Mount Doom",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(
            cost!(TapSelf, PayLife(1)),
            &[Effect::mana_choice(&[ManaColor::Black, ManaColor::Red])]
        ),
        activated!(
            cost!("{1}{B}{R}", TapSelf),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::EachOpponent),
            }]
        ),
        // NOT SUPPORTED: "{5}{B}{R}, {T}, Sacrifice Mount Doom and a
        // legendary artifact: Choose up to two creatures, then destroy the
        // rest. Activate only as a sorcery." The cost is sayable
        // (SacrificeSelf plus Sacrifice of a legendary artifact) and so is
        // the sorcery-speed timing, but the effect is not: the DSL has
        // DestroyAll, DestroyChosenForPlayers and SacrificeFilter, and none
        // of them chooses a subset of creatures and destroys the remainder.
    ],
);
