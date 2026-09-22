//! Wrenn and Realmbreaker — {1}{G}{G} — Legendary Planeswalker — Wrenn
//! Oracle: Lands you control have "{T}: Add one mana of any color."
//! Oracle: +1: Up to one target land you control becomes a 3/3 Elemental creature with vigilance, hexproof, and haste until your next turn. It's still a land.
//! Oracle: −2: Mill three cards. You may put a permanent card from among the milled cards into your hand.
//! Oracle: −7: You get an emblem with "You may play lands and cast permanent spells from your graveyard."
//! Set: MOM #217 — March of the Machine | Scryfall ID: 6f807d91-b157-44e8-a431-49782184f876 | Oracle ID: 4566fb92-448e-4b3f-9045-9d74323c35d1
// PARTIAL — the lands-you-control mana grant, the +1 land animation (3/3 Elemental,
// vigilance/hexproof/haste until your next turn), the −2 mill of three and the −7
// emblem's "you may play lands from your graveyard" are all built. The two clauses
// below have no DSL support.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WRENN_AND_REALMBREAKER,
    oracle_id = "4566fb92-448e-4b3f-9045-9d74323c35d1",
    scryfall_id = "6f807d91-b157-44e8-a431-49782184f876",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wrenn and Realmbreaker",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::WRENN],
        loyalty = Some(4),
    ),],
    coverage = Coverage::Partial(
        "no effect that puts a card from among the milled cards into its owner's hand; no \
         modifier that lets a player cast permanent spells out of a graveyard",
    ),
    abilities = &[
        // The same static Chromatic Lantern prints, word for word. It is not
        // restricted to lands that entered after the planeswalker, so the
        // filter is the whole class and the layer pass re-derives it.
        static_ability!(
            Filter::And(&[Filter::LAND, Filter::ControlledByYou]),
            Modifier::GrantActivated {
                cost: Cost::TAP,
                effects: ANY_COLOR_MANA,
                mana_ability: true,
            }
        ),
        loyalty!(
            1,
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilYourNextTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::ELEMENTAL),
                    Duration::UntilYourNextTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(3, 3),
                    Duration::UntilYourNextTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::VIGILANCE),
                    Duration::UntilYourNextTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::HEXPROOF),
                    Duration::UntilYourNextTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::HASTE),
                    Duration::UntilYourNextTurn,
                ),
            ],
            targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::YOUR_LAND))),
        ),
        // NOT SUPPORTED: "You may put a permanent card from among the milled cards into your
        // hand." — nothing chooses among the cards this resolution just milled, and
        // GraveyardToHand targets, so it would take any card and not only a milled one.
        loyalty!(
            -2,
            &[Effect::Mill {
                amount: Amount::Fixed(3),
                target: PlayerRel::You,
            }],
        ),
        // NOT SUPPORTED: "…and cast permanent spells from your graveyard." — no Modifier
        // grants a blanket permission to cast out of a graveyard; GrantsFlashback is per
        // card and exiles what it was cast from.
        loyalty!(
            -7,
            &[Effect::CreateEmblem {
                abilities: &[static_ability!(
                    Filter::Any,
                    Modifier::PlayLandsFromGraveyard
                )],
            }],
        ),
    ],
);
