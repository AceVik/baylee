//! Emeritus of Ideation // Ancestral Recall — {3}{U}{U} — Creature — Human Wizard // Instant
//! Oracle: Flying, ward {2}
//! Oracle: This creature enters prepared.
//! Oracle: Whenever this creature attacks, you may exile eight cards from your graveyard. If you do, this creature becomes prepared.
//! Oracle: Target player draws three cards.
//! Set: SOS #45 — Secrets of Strixhaven | Scryfall ID: 75961d36-acf6-425f-9698-0bf52af74f31 | Oracle ID: 24a29be4-5bc2-469b-b2ce-2fb305c3facb
//! Face: Emeritus of Ideation — {3}{U}{U} — Creature — Human Wizard
//! Face: Ancestral Recall — {U} — Instant
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ANCESTRAL_RECALL_SPELL: &[AbilityDef] = &[spell!(
    &[Effect::DrawCardsFor {
        amount: Amount::Fixed(3),
        who: PlayerRel::Chosen,
    }],
    targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
)];

card!(
    index = index::EMERITUS_OF_IDEATION,
    oracle_id = "24a29be4-5bc2-469b-b2ce-2fb305c3facb",
    scryfall_id = "75961d36-acf6-425f-9698-0bf52af74f31",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Emeritus of Ideation",
            mana_cost = mana!("{3}{U}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
            power = Some(5),
            toughness = Some(5),
            keywords = KeywordSet::FLYING,
            enter_modifiers = &[EnterModifier::Prepared],
        ),
        face!(
            name = "Ancestral Recall",
            mana_cost = mana!("{U}"),
            types = TypeSet::INSTANT,
            abilities = ANCESTRAL_RECALL_SPELL,
        ),
    ],
    abilities = &[
        AbilityDef::Ward { mana: 2 },
        AbilityDef::Prepared {
            card: index::EMERITUS_OF_IDEATION,
            face: 1,
        },
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::MayExileFromYourGraveyardThen {
                filter: &Filter::Any,
                count: 8,
                then: &[Effect::BecomePrepared],
            }],
        ),
    ],
);
