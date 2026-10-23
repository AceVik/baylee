//! Kor Chant — {2}{W} — Instant
//! Oracle: All damage that would be dealt this turn to target creature you control by a source of your choice is dealt to another target creature instead.
//! Set: TPR #17 — Tempest Remastered | Scryfall ID: 27e20ee1-ab61-463b-b516-0a883d9c7c39 | Oracle ID: 5b3f6817-5d7a-4d83-ad1d-df75b4e1970b

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KOR_CHANT,
    oracle_id = "5b3f6817-5d7a-4d83-ad1d-df75b4e1970b",
    scryfall_id = "27e20ee1-ab61-463b-b516-0a883d9c7c39",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Choosing a damage source and redirecting all damage it would deal to target creature you control this turn to another target creature is not supported; this spell prevents all damage that would be dealt to target creature you control this turn instead."
    ),
    faces = &[face!(
        name = "Kor Chant",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::PreventDamageToIt,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
    )],
);
