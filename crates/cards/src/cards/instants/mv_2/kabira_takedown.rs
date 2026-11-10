//! Kabira Takedown // Kabira Plateau — {1}{W} — Instant // Land
//! Oracle: Kabira Takedown deals damage equal to the number of creatures you control to target creature or planeswalker.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Set: ZNR #19 — Zendikar Rising | Scryfall ID: 366e9845-019d-47cc-adb8-8fbbaad35b6d | Oracle ID: 0bb73c07-0220-4ba9-8d85-3c357c223833
//! Face: Kabira Takedown — {1}{W} — Instant
//! Face: Kabira Plateau —  — Land
// IMPLEMENTED — the front deals damage equal to the creatures you control to
// target creature or planeswalker; the back land enters tapped and taps for {W}.

use baylee_cards_dsl::prelude::*;

/// `{T}: Add {W}.` — the back face's own mana ability, because a face with no
/// basic land type has no intrinsic one to fall back on.
static PLATEAU_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])];

card!(
    index = index::KABIRA_TAKEDOWN,
    oracle_id = "0bb73c07-0220-4ba9-8d85-3c357c223833",
    scryfall_id = "366e9845-019d-47cc-adb8-8fbbaad35b6d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Kabira Takedown",
            mana_cost = mana!("{1}{W}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Kabira Plateau",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = PLATEAU_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::CountOf {
                filter: &Filter::YOUR_CREATURE,
                zone: ZoneSel::Battlefield,
            },
            target: TargetSpec::Object(&Filter::CREATURE_OR_PLANESWALKER),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::CREATURE_OR_PLANESWALKER
        )))
    )],
);
