//! Umara Wizard // Umara Skyfalls — {4}{U} — Creature — Merfolk Wizard // Land
//! Oracle: Whenever you cast an instant, sorcery, or Wizard spell, this creature gains flying until end of turn.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Set: ZNR #86 — Zendikar Rising | Scryfall ID: 890eee8d-a339-4143-adfa-1b17ec10c099 | Oracle ID: 6bc668f4-8fc7-4aaf-891b-277d8328b376
//! Face: Umara Wizard — {4}{U} — Creature — Merfolk Wizard
//! Face: Umara Skyfalls —  — Land
// IMPLEMENTED — a Merfolk Wizard that gains flying until end of turn every
// time you cast an instant, a sorcery or a Wizard spell; the back is an MDFC
// land reached by the face choice on a land play (CR 712.12), which comes
// down tapped and taps for {U}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "An instant, sorcery, or Wizard spell" that *you* cast.
///
/// The trigger reads the spell while it is on the stack, so the controller
/// clause is the whole of "you cast" and nothing else is needed to keep an
/// opponent's spell out — the same shape Storm-Kiln Artist and Jin-Gitaxias
/// use. `HasSubtype` reads the printed tribe of a spell on the stack exactly
/// as it would on the battlefield, which is what makes the third clause a
/// *Wizard* spell rather than a Wizard permanent.
static YOUR_INSTANT_SORCERY_OR_WIZARD: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[
        Filter::HasType(TypeSet::INSTANT),
        Filter::HasType(TypeSet::SORCERY),
        Filter::HasSubtype(subtypes::creature::WIZARD),
    ]),
]);

/// The back face's whole printed text apart from the tapland clause, which
/// is an `EnterModifier` rather than an ability.
static SKYFALLS_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])];

card!(
    index = index::UMARA_WIZARD,
    oracle_id = "6bc668f4-8fc7-4aaf-891b-277d8328b376",
    scryfall_id = "890eee8d-a339-4143-adfa-1b17ec10c099",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Umara Wizard",
            mana_cost = mana!("{4}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::WIZARD],
            power = Some(4),
            toughness = Some(3),
        ),
        face!(
            name = "Umara Skyfalls",
            types = TypeSet::LAND,
            abilities = SKYFALLS_MANA,
            enter_modifiers = &[EnterModifier::Tapped],
        ),
    ],
    coverage = Coverage::Implemented,
    // The trigger names no target, so `Filter::This` inside the continuous
    // effect is the source and not a chosen object (`Resolution::targeted`
    // is what tells the two apart) — which is what "this creature gains
    // flying" says. A `PumpTarget` would need a target the card never asks
    // for.
    abilities = &[triggered!(
        Trigger::SpellCast(&YOUR_INSTANT_SORCERY_OR_WIZARD),
        &[Effect::continuous(
            &Filter::This,
            Modifier::AddKeyword(KeywordSet::FLYING),
            Duration::UntilEndOfTurn
        )]
    )],
);

// Behaviour belongs in `baylee-engine`'s `card_tests`: with this on the
// battlefield, casting an instant, a sorcery or a Wizard creature spell each
// give it flying for the turn and a vanilla creature spell gives it none;
// the back face is played as a land that arrives tapped and taps for {U}.
