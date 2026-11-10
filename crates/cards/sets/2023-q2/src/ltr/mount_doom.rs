//! Mount Doom — (no cost) — Legendary Land
//! Oracle: {T}, Pay 1 life: Add {B} or {R}.
//! Oracle: {1}{B}{R}, {T}: Mount Doom deals 1 damage to each opponent.
//! Oracle: {5}{B}{R}, {T}, Sacrifice Mount Doom and a legendary artifact: Choose up to two creatures, then destroy the rest. Activate only as a sorcery.
//! Set: LTR #258 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: b5bc71a1-2344-4bc6-aa60-658cec19d0d6 | Oracle ID: 995c8dac-fd27-468a-abd4-02372cf0c850
// IMPLEMENTED — {T}, Pay 1 life: Add {B} or {R}; {1}{B}{R}, {T}: 1 damage to
// each opponent; {5}{B}{R}, {T}, Sacrifice this and a legendary artifact:
// choose up to two creatures and destroy the rest, at sorcery speed.

use baylee_cards_dsl::prelude::*;

static LEGENDARY_ARTIFACT_YOU_CONTROL: Filter = Filter::And(&[
    Filter::ARTIFACT,
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::ControlledByYou,
]);

card!(
    index = index::MOUNT_DOOM,
    oracle_id = "995c8dac-fd27-468a-abd4-02372cf0c850",
    scryfall_id = "b5bc71a1-2344-4bc6-aa60-658cec19d0d6",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
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
        // "Sacrifice Mount Doom and a legendary artifact" is two cost parts:
        // the land cannot pay for the artifact, and Mount Doom is no
        // artifact, so the two answers never collide. "Choose up to two
        // creatures, then destroy the rest" targets nothing, so a creature
        // with hexproof or shroud may be kept or destroyed alike.
        activated!(
            cost!(
                "{5}{B}{R}",
                TapSelf,
                SacrificeSelf,
                Sacrifice(&LEGENDARY_ARTIFACT_YOU_CONTROL)
            ),
            &[Effect::DestroyAllExceptChosen {
                filter: &Filter::CREATURE,
                keep: 2,
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
