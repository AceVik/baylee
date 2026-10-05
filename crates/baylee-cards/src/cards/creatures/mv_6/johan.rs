//! Johan — {3}{R}{G}{W} — Legendary Creature — Human Wizard
//! Oracle: At the beginning of combat on your turn, you may have Johan gain "Johan can't attack" until end of combat. If you do, attacking doesn't cause creatures you control to tap this combat if Johan is untapped.
//! Set: CHR #77 — Chronicles | Scryfall ID: 2f2f3b3e-63f3-4cab-aa95-030990157ed5 | Oracle ID: ad33ada7-4f89-4cab-88db-1f4d024baa36
// PARTIAL — the beginning-of-combat choice is written; the untapped-gated vigilance half is off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JOHAN,
    oracle_id = "ad33ada7-4f89-4cab-88db-1f4d024baa36",
    scryfall_id = "2f2f3b3e-63f3-4cab-aa95-030990157ed5",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "the second sentence is gated on Johan being untapped as the rule \
         applies, and a created continuous effect carries no condition: \
         granting vigilance without it would be strictly stronger than \
         the printed card"
    ),
    faces = &[face!(
        name = "Johan",
        mana_cost = mana!("{3}{R}{G}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(5),
        toughness = Some(4),
    ),],
    // NOT SUPPORTED: "If you do, attacking doesn't cause creatures you
    // control to tap this combat if Johan is untapped." — the mass half is
    // expressible as `AddKeyword(VIGILANCE)` until end of combat, but "if
    // Johan is untapped" is a condition re-read as the rule applies, and
    // `Effect::CreateContinuousEffect` has no `condition` field (only a
    // printed `StaticAbility` does, and a printed static cannot be gated on
    // this trigger's choice or last only this combat). Granting it anyway
    // would hand out the vigilance after Johan is tapped.
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatBegin,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::CANT_ATTACK),
                Duration::UntilEndOfCombat
            )],
        }]
    )],
);
