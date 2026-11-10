//! Muscle Burst — {1}{G} — Instant
//! Oracle: Target creature gets +X/+X until end of turn, where X is 3 plus the number of cards named Muscle Burst in all graveyards.
//! Set: ODY #252 — Odyssey | Scryfall ID: 217dada5-7ffc-488b-8062-34c034906ea9 | Oracle ID: 97487ea5-2bbd-4ef6-a870-7e9f2db5e5e0
// IMPLEMENTED — +X/+X until end of turn, X = 3 plus the copies of this card
// in every graveyard. The spell itself is still on the stack while it
// resolves (CR 608.2m), so it never counts itself.

use baylee_cards_dsl::prelude::*;

/// The other copies: cards *named* Muscle Burst, in anybody's graveyard.
///
/// CR 201.2 compares the name an object has now, so this counts a card that
/// was named this way by a text-changing effect and skips one that renamed
/// itself away — which is what a printed "cards named …" asks for.
static COPIES: Amount = Amount::Plus {
    base: &Amount::CountOf {
        filter: &Filter::Named("Muscle Burst"),
        zone: ZoneSel::GraveyardAll,
    },
    offset: 3,
};

card!(
    index = index::MUSCLE_BURST,
    oracle_id = "97487ea5-2bbd-4ef6-a870-7e9f2db5e5e0",
    scryfall_id = "217dada5-7ffc-488b-8062-34c034906ea9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Muscle Burst",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: COPIES,
            toughness: COPIES,
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),],
);
