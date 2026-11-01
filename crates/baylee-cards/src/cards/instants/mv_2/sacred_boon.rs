//! Sacred Boon — {1}{W} — Instant
//! Oracle: Prevent the next 3 damage that would be dealt to target creature this turn. At the beginning of the next end step, put a +0/+1 counter on that creature for each 1 damage prevented this way.
//! Set: ME2 #33 — Masters Edition II | Scryfall ID: f8086253-5a66-4161-bdef-f460851368ed | Oracle ID: 32a8d49b-cdfb-4944-bfb5-afd704cc3658
use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "At the beginning of the next end step, put a +0/+1 counter on that creature for each 1 damage prevented this way." Prevention shields do not record how much damage they prevented for a later end-step delayed trigger.
card!(
    index = index::SACRED_BOON,
    oracle_id = "32a8d49b-cdfb-4944-bfb5-afd704cc3658",
    scryfall_id = "f8086253-5a66-4161-bdef-f460851368ed",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "prevents the next 3 damage that would be dealt to target creature this turn; putting +0/+1 counters on that creature at the next end step for each 1 damage prevented this way is not supported",
    ),
    faces = &[face!(
        name = "Sacred Boon",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PreventNextDamage {
            target: TargetSpec::Object(&Filter::CREATURE),
            amount: Amount::Fixed(3),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),],
);
