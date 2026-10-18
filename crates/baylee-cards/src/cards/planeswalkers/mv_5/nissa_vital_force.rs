//! Nissa, Vital Force — {3}{G}{G} — Legendary Planeswalker — Nissa
//! Oracle: +1: Untap target land you control. Until your next turn, it becomes a 5/5 Elemental creature with haste. It's still a land.
//! Oracle: −3: Return target permanent card from your graveyard to your hand.
//! Oracle: −6: You get an emblem with "Whenever a land you control enters, you may draw a card."
//! Set: KLD #163 — Kaladesh | Scryfall ID: bbaaa98a-ec40-4ff1-8762-a719cf1c475d | Oracle ID: 31d4d8ce-36ba-4297-8c7a-34a83ba25255
// IMPLEMENTED — +1 untap + 5/5 Elemental haste land animation until next turn, −3 permanent card recursion, −6 landfall-draw emblem.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PERMANENT_CARD: Filter = Filter::HasType(
    TypeSet::ARTIFACT
        .union(TypeSet::BATTLE)
        .union(TypeSet::CREATURE)
        .union(TypeSet::ENCHANTMENT)
        .union(TypeSet::LAND)
        .union(TypeSet::PLANESWALKER),
);

static GRAVEYARD_PERMANENT: TargetSpec =
    TargetSpec::CardInGraveyard(&PERMANENT_CARD, PlayerRel::You);

static EMBLEM_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::EntersBattlefield(&Filter::YOUR_LAND),
    &[Effect::MayDo {
        effects: &[Effect::draw(1)],
    }],
)];

card!(
    index = index::NISSA_VITAL_FORCE,
    oracle_id = "31d4d8ce-36ba-4297-8c7a-34a83ba25255",
    scryfall_id = "bbaaa98a-ec40-4ff1-8762-a719cf1c475d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Nissa, Vital Force",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::NISSA],
        loyalty = Some(5),
    ),],
    abilities = &[
        loyalty!(
            1,
            &[
                Effect::UntapTarget,
                Effect::ModifyTargets {
                    modifier: Modifier::AddType(TypeSet::CREATURE),
                    duration: Duration::UntilYourNextTurn,
                },
                Effect::ModifyTargets {
                    modifier: Modifier::AddSubtype(subtypes::creature::ELEMENTAL),
                    duration: Duration::UntilYourNextTurn,
                },
                Effect::ModifyTargets {
                    modifier: Modifier::AddKeyword(KeywordSet::HASTE),
                    duration: Duration::UntilYourNextTurn,
                },
                Effect::ModifyTargets {
                    modifier: Modifier::SetPT(5, 5),
                    duration: Duration::UntilYourNextTurn,
                },
            ],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_LAND))),
        ),
        loyalty!(
            -3,
            &[Effect::GraveyardToHand {
                target: GRAVEYARD_PERMANENT,
            }],
            targets = Some(TargetReq::one(GRAVEYARD_PERMANENT)),
        ),
        loyalty!(
            -6,
            &[Effect::CreateEmblem {
                abilities: EMBLEM_ABILITIES,
            }],
        ),
    ],
);
