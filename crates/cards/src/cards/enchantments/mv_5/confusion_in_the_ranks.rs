//! Confusion in the Ranks — {3}{R}{R} — Enchantment
//! Oracle: Whenever an artifact, creature, or enchantment enters, its controller chooses target permanent another player controls that shares a card type with it. Exchange control of those permanents.
//! Set: MRD #87 — Mirrodin | Scryfall ID: 9d46aca2-4714-4d2c-9466-5f1c043b8726 | Oracle ID: e477abb9-9b99-48a8-8c09-07b120b72540
use baylee_cards_dsl::prelude::*;

static TRIGGER_FILTER: Filter = Filter::ARTIFACT_CREATURE_OR_ENCHANTMENT;
static TARGET_FILTER: Filter = Filter::And(&[
    Filter::NotControlledByReferenceController,
    Filter::SharesTypeWithReference(
        TypeSet::ARTIFACT
            .union(TypeSet::CREATURE)
            .union(TypeSet::ENCHANTMENT),
    ),
]);

card!(
    index = index::CONFUSION_IN_THE_RANKS,
    oracle_id = "e477abb9-9b99-48a8-8c09-07b120b72540",
    scryfall_id = "9d46aca2-4714-4d2c-9466-5f1c043b8726",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Confusion in the Ranks",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&TRIGGER_FILTER),
        &[Effect::ExchangeControlOfEventAndTarget],
        targets = Some(TargetReq::one(TargetSpec::Object(&TARGET_FILTER)).chosen_by_event_player()),
    )],
);
