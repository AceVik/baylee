//! Aclazotz, Deepest Betrayal // Temple of the Dead — {3}{B}{B} — Legendary Creature — Bat God // Land
//! Set: LCI #88 — The Lost Caverns of Ixalan | Scryfall ID: 627c392c-4d18-4eb2-a4e8-c668f61f5487 | Oracle ID: fcdfe9d5-2743-4d3e-ab57-bf0f96beaa15
//! Face: Aclazotz, Deepest Betrayal — {3}{B}{B} — Legendary Creature — Bat God
//! Face: Temple of the Dead —  — Land
// PARTIAL — attacks discard implemented; draw-if-can't-discard, land-discard→Bat-token,
// and die→return-tapped-transformed not supported (no Discards trigger, no Bat token,
// ExileSelfReturnAsFace inapplicable from graveyard); Temple transform condition
// (player has ≤1 card in hand) not in ActivationCondition.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// ── Temple of the Dead (face 1) abilities ────────────────────────────────────

static TEMPLE_ABILITIES: &[AbilityDef] = &[
    // {T}: Add {B}.
    mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
    // {2}{B}, {T}: Transform this land. Activate only as a sorcery.
    // NOT SUPPORTED: activation condition "only if a player has one or fewer cards in hand"
    // — no ActivationCondition variant for hand-count check; implemented unconditionally.
    activated!(
        Cost {
            mana: baylee_core::mana!("{2}{B}"),
            parts: &[CostPart::TapSelf],
        },
        &[Effect::ExileSelfReturnAsFace { face: 0 }],
        timing: ActivationTiming::SorcerySpeed,
    ),
];

// ── Card ─────────────────────────────────────────────────────────────────────

card! {
    index: 205,
    oracle_id: "fcdfe9d5-2743-4d3e-ab57-bf0f96beaa15",
    scryfall_id: "627c392c-4d18-4eb2-a4e8-c668f61f5487",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    commander: CommanderRule::Legendary,
    // Flying and lifelink are keywords on face 0 (Aclazotz, Deepest Betrayal).
    keywords: KeywordSet::FLYING.union(KeywordSet::LIFELINK),
    faces: &[
        face! {
            name: "Aclazotz, Deepest Betrayal",
            mana_cost: baylee_core::mana!("{3}{B}{B}"),
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::LEGENDARY,
            subtypes: &[subtypes::creature::BAT, subtypes::creature::GOD],
            power: Some(4),
            toughness: Some(4),
        },
        face! {
            name: "Temple of the Dead",
            types: TypeSet::LAND,
            castable_from_hand: false,
            abilities: TEMPLE_ABILITIES,
        },
    ],
    coverage: Coverage::Partial("no Discards trigger (Bat-token clause inexpressible); draw-if-cant-discard not supported; ExileSelfReturnAsFace inapplicable from graveyard (die→return-tapped-transformed); ActivationCondition has no hand-count check (Temple transform condition)"),
    abilities: &[
        // "Whenever Aclazotz attacks, each opponent discards a card."
        // NOT SUPPORTED (partial): "For each opponent who can't, you draw a card."
        // — no DSL primitive for drawing per failed discard; the discard half is
        // implemented but the draw-on-failure clause is omitted.
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::DiscardForPlayers {
                who: PlayerRel::EachOpponent,
                count: 1,
            }]
        ),
        // NOT SUPPORTED: "Whenever an opponent discards a land card, create a 1/1
        // black Bat creature token with flying." — Trigger::Discards does not exist;
        // no Bat token is defined in crate::tokens.
        //
        // NOT SUPPORTED: "When Aclazotz dies, return it to the battlefield tapped
        // and transformed under its owner's control." — ExileSelfReturnAsFace
        // cannot operate from the graveyard (the permanent has already moved there
        // when the trigger resolves), and no "tapped" rider exists on that effect.
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_matches_registry() {
        assert_eq!(CARD.index.get(), 205);
    }

    #[test]
    fn oracle_and_scryfall_ids() {
        assert_eq!(CARD.oracle_id, "fcdfe9d5-2743-4d3e-ab57-bf0f96beaa15");
        assert_eq!(CARD.scryfall_id, "627c392c-4d18-4eb2-a4e8-c668f61f5487");
    }

    #[test]
    fn front_face_is_legendary_bat_god_creature() {
        let face = &CARD.faces[0];
        assert_eq!(face.name, "Aclazotz, Deepest Betrayal");
        assert!(face.types.contains(TypeSet::CREATURE));
        assert!(face.supertypes.contains(SupertypeSet::LEGENDARY));
        assert_eq!(face.power, Some(4));
        assert_eq!(face.toughness, Some(4));
    }

    #[test]
    fn back_face_is_land() {
        let face = &CARD.faces[1];
        assert_eq!(face.name, "Temple of the Dead");
        assert!(face.types.contains(TypeSet::LAND));
        assert!(!face.castable_from_hand);
    }

    #[test]
    fn color_identity_is_black() {
        assert_eq!(CARD.color_identity, ColorSet::from_slice(&[Color::Black]));
    }

    #[test]
    fn keywords_include_flying_and_lifelink() {
        assert!(CARD.keywords.contains(KeywordSet::FLYING));
        assert!(CARD.keywords.contains(KeywordSet::LIFELINK));
    }

    #[test]
    fn coverage_is_partial() {
        assert!(matches!(CARD.coverage, Coverage::Partial(_)));
    }

    #[test]
    fn commander_eligible_as_legendary() {
        assert!(matches!(CARD.commander, CommanderRule::Legendary));
    }

    // Engine-level tests belong in baylee-engine:
    // aclazotz_attacks_forces_discard — attacks trigger fires, each opponent
    //   discards one card.
    // temple_of_the_dead_taps_for_black — {T}: Add {B}.
    // temple_transforms_back_to_aclazotz — {2}{B},{T} at sorcery speed returns
    //   to face 0 (condition guard missing until ActivationCondition gains hand-count).
}
