//! Runed Arch — {3} — Artifact
//! Oracle: This artifact enters tapped.
//! Oracle: {X}, {T}, Sacrifice this artifact: X target creatures with power 2 or less can't be blocked this turn.
//! Set: ICE #334 — Ice Age | Scryfall ID: ca02861b-9639-480d-8e54-e024f0c70158 | Oracle ID: 32717625-46c6-45c5-ad0a-fb6a91f9878c
use baylee_cards_dsl::prelude::*;

static SMALL_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::PowerAtMost(2)]);

card!(
    index = index::RUNED_ARCH,
    oracle_id = "32717625-46c6-45c5-ad0a-fb6a91f9878c",
    scryfall_id = "ca02861b-9639-480d-8e54-e024f0c70158",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Runed Arch",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[activated!(
        cost!("{X}", TapSelf, SacrificeSelf),
        &[Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::UNBLOCKABLE,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&SMALL_CREATURE))),
    ),],
);
