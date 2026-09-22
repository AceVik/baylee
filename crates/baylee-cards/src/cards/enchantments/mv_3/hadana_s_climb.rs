//! Hadana's Climb // Winged Temple of Orazca — {1}{G}{U} — Legendary Enchantment // Legendary Land
//! Oracle: At the beginning of combat on your turn, put a +1/+1 counter on target creature you control. Then if that creature has three or more +1/+1 counters on it, transform Hadana's Climb.
//! Oracle: (Transforms from Hadana's Climb.)
//! Oracle: {T}: Add one mana of any color.
//! Oracle: {1}{G}{U}, {T}: Target creature you control gains flying and gets +X/+X until end of turn, where X is its power.
//! Set: RIX #158 — Rivals of Ixalan | Scryfall ID: 8e7554bc-8583-4059-8895-c3845bc27ae3 | Oracle ID: 93b91d18-6acf-42e5-9a31-bc6e01f90c1f
//! Face: Hadana's Climb — {1}{G}{U} — Legendary Enchantment
//! Face: Winged Temple of Orazca —  — Legendary Land
// PARTIAL — the combat trigger's counter, the back face's mana ability and its
// pump are built; the transform clause is not (see the NOT SUPPORTED line).

use baylee_cards_dsl::prelude::*;

/// The back face's own abilities — Winged Temple of Orazca's mana line and its
/// pump. A back face reads what it prints; the front face's abilities are the
/// card-level list.
static BACK_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana_of_any_color()]),
    activated!(
        cost!("{1}{G}{U}", TapSelf),
        &[Effect::PumpTarget {
            power: Amount::TargetPower,
            toughness: Amount::TargetPower,
            keywords: KeywordSet::FLYING,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE))
    ),
];

card!(
    index = index::HADANA_S_CLIMB,
    oracle_id = "93b91d18-6acf-42e5-9a31-bc6e01f90c1f",
    scryfall_id = "8e7554bc-8583-4059-8895-c3845bc27ae3",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[
        face!(
            name = "Hadana's Climb",
            mana_cost = mana!("{1}{G}{U}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Winged Temple of Orazca",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = BACK_ABILITIES,
        ),
    ],
    coverage = Coverage::Partial(
        "front face: \"Then if that creature has three or more +1/+1 counters on it, \
         transform Hadana's Climb\" — no effect reads a counter count off the \
         ability's target"
    ),
    abilities = &[
        // NOT SUPPORTED: "Then if that creature has three or more +1/+1 counters
        // on it, transform Hadana's Climb." — the count is asked of the ability's
        // *target*, while `Condition::CountersOnSelf` reads the source and no
        // `Effect` carries a per-target test. The counter half is built; the
        // transform never happens.
        triggered!(
            Trigger::StepBegin {
                step: StepKind::CombatBegin,
                whose: PlayerRel::You,
            },
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE)))
        ),
    ],
);
