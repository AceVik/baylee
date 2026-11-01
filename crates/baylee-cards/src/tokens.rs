//! Central token definitions — one constant per named token, referenced
//! by card files instead of duplicating `TokenDef` literals. Each token
//! has a stable `token_id` (its index in [`ALL`]) so clients can map it
//! to token art.
//!
//! Add new tokens here (keep `ALL` in sync — the id IS the art key).
//!
//! # Why a token may not be defined in a card file
//!
//! The index into [`ALL`] is what the engine stamps on the object it creates
//! and what the view hands the client, so a `TokenDef` that lives anywhere
//! else has no id: [`token_id`] answers `u16::MAX` for it, and the token
//! reaches the table nameless as far as art is concerned. Two cards did
//! exactly that and quietly lost their tokens' identity. The test at the foot
//! of this file is what turns the convention into a build failure.
//!
//! # Abilities
//!
//! A token carries its own abilities ([`TokenDef::abilities`]), read by the
//! engine through the same path a card face's are. That is what makes a
//! Treasure a Treasure rather than a blank artifact: before it existed, every
//! token on the battlefield was inert whatever its name said.

use baylee_cards_dsl::{
    AbilityDef, Effect, Filter, KeywordSet, Modifier, PtCount, TokenDef, Trigger, activated, cost,
    mana_ability, static_ability, triggered,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{artifact, creature};
use baylee_core::mana::ManaColor;
use baylee_core::types::TypeSet;

/// The five colors, for "any color" mana abilities.
static ANY_COLOR: &[ManaColor] = &[
    ManaColor::White,
    ManaColor::Blue,
    ManaColor::Black,
    ManaColor::Red,
    ManaColor::Green,
];

/// `{T}, Sacrifice this artifact: Add one mana of any color.` (Treasure)
// A mana ability (CR 605.1a): no target, adds mana, is not itself an
// activated ability that uses the stack — which is what lets a Treasure be
// cracked while paying for a spell. `mana_ability!` is that statement; the
// flag it sets is the one a card must never write by hand.
static SACRIFICE_FOR_ANY_COLOR: &[AbilityDef] = &[mana_ability!(
    cost!(TapSelf, SacrificeSelf),
    &[Effect::mana_choice(ANY_COLOR)]
)];

/// `{2}, Sacrifice this artifact: Draw a card.` (Clue)
static SACRIFICE_TO_DRAW: &[AbilityDef] =
    &[activated!(cost!("{2}", SacrificeSelf), &[Effect::draw(1)])];

/// `{2}, {T}, Sacrifice this token: You gain 3 life.` (Food)
static SACRIFICE_TO_GAIN_LIFE: &[AbilityDef] = &[activated!(
    cost!("{2}", TapSelf, SacrificeSelf),
    &[Effect::gain_life(3)]
)];

/// `{1}, {T}, Discard a card, Sacrifice this artifact: Draw a card.` (Blood)
static SACRIFICE_TO_LOOT: &[AbilityDef] = &[activated!(
    cost!("{1}", TapSelf, Discard(&Filter::Any), SacrificeSelf),
    &[Effect::draw(1)]
)];

/// 1/1 white Ally (Aang and Katara, Jasmine Dragon Tea Shop, Sokka).
pub static ALLY_1_1_WHITE: TokenDef = TokenDef {
    name: "Ally",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::ALLY],
    power: Some(1),
    toughness: Some(1),
    // TLA #8 — Avatar: The Last Airbender, the set the three cards that
    // make this token come from.
    scryfall_id: "01439983-8394-4a0c-9e9e-92a3f1927fe3",
    ..TokenDef::DEFAULT
};

/// 4/4 white Angel with flying (Luminarch Ascension).
pub static ANGEL_4_4_WHITE_FLYING: TokenDef = TokenDef {
    name: "Angel",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::ANGEL],
    power: Some(4),
    toughness: Some(4),
    keywords: KeywordSet::FLYING,
    // SOI #1 — flying and nothing else. Most Angel tokens in print are
    // 4/4 flying *and vigilance*, and their picture says so on the card.
    scryfall_id: "bdb975fe-ac30-4249-bb55-7efb64645e4d",
    ..TokenDef::DEFAULT
};

/// 0/0 black Army (amass, CR 701.47a — the token the mechanic creates when
/// you control no Army; the counters go on afterwards, and "amass Orcs"
/// adds the Orc type on top of this).
pub static ARMY_0_0_BLACK: TokenDef = TokenDef {
    name: "Orc Army",
    colors: ColorSet::from_slice(&[Color::Black]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::ORC, creature::ARMY],
    power: Some(0),
    toughness: Some(0),
    // LTR #6: amass Orcs creates both subtypes before the entry event.
    scryfall_id: "db598f33-2ff9-4e0b-a067-05fecc03435f",
    ..TokenDef::DEFAULT
};

/// 5/5 green Beast (One Dozen Eyes). The printing used for art also has
/// trample, which One Dozen Eyes does not grant.
pub static BEAST_5_5_GREEN: TokenDef = TokenDef {
    name: "Beast",
    colors: ColorSet::from_slice(&[Color::Green]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::BEAST],
    power: Some(5),
    toughness: Some(5),
    // C18 #14 — the only 5/5 Beast token printing Scryfall indexes.
    scryfall_id: "44675b6a-ea5a-44f1-9f2c-3725cdfcc814",
    ..TokenDef::DEFAULT
};

/// 1/1 white Bird with flying (Aether Channeler).
pub static BIRD_1_1_WHITE_FLYING: TokenDef = TokenDef {
    name: "Bird",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::BIRD],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::FLYING,
    // DMU #2, Aether Channeler's own set.
    scryfall_id: "5f3034f6-145f-4e60-9e55-c4054fd8e70f",
    ..TokenDef::DEFAULT
};

/// Colorless Blood artifact: `{1}, {T}, Discard a card, Sacrifice this
/// artifact: Draw a card.`
pub static BLOOD: TokenDef = TokenDef {
    name: "Blood",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::BLOOD],
    abilities: SACRIFICE_TO_LOOT,
    // VOW #17, the set that introduced Blood.
    scryfall_id: "a6f374bc-cd29-469f-808a-6a6c004ee8aa",
    ..TokenDef::DEFAULT
};

/// 2/2 green Boar (Curse of the Swine).
pub static BOAR_2_2_GREEN: TokenDef = TokenDef {
    name: "Boar",
    colors: ColorSet::from_slice(&[Color::Green]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::BOAR],
    power: Some(2),
    toughness: Some(2),
    // THS #8, Curse of the Swine's own set.
    scryfall_id: "2f40613b-1bde-4939-86ad-6bd40f9db0d6",
    ..TokenDef::DEFAULT
};

/// Colorless Clue artifact: `{2}, Sacrifice this artifact: Draw a card.`
pub static CLUE: TokenDef = TokenDef {
    name: "Clue",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::CLUE],
    abilities: SACRIFICE_TO_DRAW,
    // SOI #13, the set that introduced Clues.
    scryfall_id: "271afa7e-2126-4497-b871-9795b7355d69",
    ..TokenDef::DEFAULT
};

/// 0/0 colorless Construct artifact creature (Urza's Saga). Its size comes
/// from a continuous effect the card registers, not from the token.
pub static CONSTRUCT_ARTIFACT_0_0: TokenDef = TokenDef {
    name: "Construct",
    types: TypeSet::CREATURE.union(TypeSet::ARTIFACT),
    subtypes: &[creature::CONSTRUCT],
    power: Some(0),
    toughness: Some(0),
    // MH2 #16, Urza's Saga's own set.
    scryfall_id: "a7caaf39-8f16-4f1d-bee6-a45674306319",
    ..TokenDef::DEFAULT
};

/// "This token's power and toughness are each equal to the number of
/// creatures you control." A characteristic-defining ability (CR 604.3),
/// applied in layer 7a; the token counts itself.
static POWER_IS_CREATURES_YOU_CONTROL: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::CharacteristicPT {
        count: PtCount::YouControl(&Filter::CREATURE),
        toughness_plus: 0,
    }
)];

/// Green and white Elemental whose power and toughness are each the number
/// of creatures its controller controls (Voice of Resurgence). It prints
/// `*/*`, so it has no base size: the ability is the whole of it.
pub static ELEMENTAL_X_X_GREEN_WHITE: TokenDef = TokenDef {
    name: "Elemental",
    colors: ColorSet::from_slice(&[Color::Green, Color::White]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::ELEMENTAL],
    abilities: POWER_IS_CREATURES_YOU_CONTROL,
    // 2XM #20, Voice of Resurgence's own set.
    scryfall_id: "8676704a-419e-4a00-a052-bca2ad34ecae",
    ..TokenDef::DEFAULT
};

/// Colorless Food artifact: `{2}, {T}, Sacrifice this token: You gain
/// 3 life.`
pub static FOOD: TokenDef = TokenDef {
    name: "Food",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::FOOD],
    abilities: SACRIFICE_TO_GAIN_LIFE,
    // ELD #15, the set that introduced Food.
    scryfall_id: "bf36408d-ed85-497f-8e68-d3a922c388a0",
    ..TokenDef::DEFAULT
};

/// `Whenever this creature attacks, create a Treasure token.` (the Goblin
/// Shaman below)
static ATTACKS_FOR_TREASURE: &[AbilityDef] = &[triggered!(
    Trigger::Attacks(&Filter::This),
    &[Effect::CreateToken { token: &TREASURE }]
)];

/// 2/2 red Goblin Shaman with "Whenever this creature attacks, create a
/// Treasure token." (Fable of the Mirror-Breaker's chapter I)
pub static GOBLIN_SHAMAN_2_2_RED: TokenDef = TokenDef {
    name: "Goblin Shaman",
    colors: ColorSet::from_slice(&[Color::Red]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::GOBLIN, creature::SHAMAN],
    power: Some(2),
    toughness: Some(2),
    abilities: ATTACKS_FOR_TREASURE,
    // TNEO #8, the token Fable of the Mirror-Breaker's own printing names.
    scryfall_id: "0d9461c3-f545-4efb-926e-759961db0495",
    ..TokenDef::DEFAULT
};

/// Blue Illusion with no printed size (Skyclave Apparition): the card sets
/// its power and toughness to the exiled card's mana value as it is created,
/// so leaving them unset here is what lets that effect speak.
pub static ILLUSION_X_BLUE: TokenDef = TokenDef {
    name: "Illusion",
    colors: ColorSet::from_slice(&[Color::Blue]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::ILLUSION],
    // ZNR #6, Skyclave Apparition's own set — and the one Illusion token
    // in print with no fixed size, which is what this token is.
    scryfall_id: "2300635e-7771-4676-a5a5-29a9d8f49f1a",
    ..TokenDef::DEFAULT
};

/// 1/1 colorless Shapeshifter with changeling (Crib Swap).
pub static SHAPESHIFTER_1_1_CHANGELING: TokenDef = TokenDef {
    name: "Shapeshifter",
    types: TypeSet::CREATURE,
    subtypes: &[creature::SHAPESHIFTER],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::CHANGELING,
    // LRW #11, Crib Swap's own set.
    scryfall_id: "1a7d89ca-8611-4bda-b5c8-0350ce091102",
    ..TokenDef::DEFAULT
};

/// 2/2 blue Shapeshifter with changeling (Maskwood Nexus).
pub static SHAPESHIFTER_2_2_BLUE_CHANGELING: TokenDef = TokenDef {
    name: "Shapeshifter",
    colors: ColorSet::from_slice(&[Color::Blue]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::SHAPESHIFTER],
    power: Some(2),
    toughness: Some(2),
    keywords: KeywordSet::CHANGELING,
    // KHM #8, Maskwood Nexus's own set.
    scryfall_id: "ef775ad0-b1a9-4254-ab6f-304558bb77a1",
    ..TokenDef::DEFAULT
};

/// 1/1 white Soldier (Elspeth, Storm Slayer).
pub static SOLDIER_1_1_WHITE: TokenDef = TokenDef {
    name: "Soldier",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::SOLDIER],
    power: Some(1),
    toughness: Some(1),
    // DMU #4. Elspeth's own set prints no Soldier token of its own, and a
    // 1/1 white Soldier is the same card in every set that does.
    scryfall_id: "8c4b0257-2ca5-4015-9d63-d7cf6e87ab9d",
    ..TokenDef::DEFAULT
};

/// Colorless Treasure artifact: `{T}, Sacrifice this artifact: Add one mana
/// of any color.` (Smothering Tithe)
pub static TREASURE: TokenDef = TokenDef {
    name: "Treasure",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::TREASURE],
    abilities: SACRIFICE_FOR_ANY_COLOR,
    // RNA #12, Smothering Tithe's own set.
    scryfall_id: "0559f9f3-eff0-465d-93c1-e875a8afe87f",
    ..TokenDef::DEFAULT
};

/// 0/1 black Wizard created by Mage Siege, with its noncreature-spell trigger.
pub static WIZARD_0_1_BLACK: TokenDef = TokenDef {
    name: "Wizard",
    colors: ColorSet::from_slice(&[Color::Black]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::WIZARD],
    power: Some(0),
    toughness: Some(1),
    abilities: &[triggered!(
        Trigger::SpellCast(&Filter::And(&[
            Filter::ControlledByYou,
            Filter::NONCREATURE,
        ])),
        &[Effect::DealDamage {
            amount: baylee_cards_dsl::Amount::Fixed(1),
            target: baylee_cards_dsl::TargetSpec::Player(baylee_cards_dsl::PlayerRel::EachOpponent),
        }]
    )],
    // TFIN #14; verified against the Scryfall API. Art is fetched by the client.
    scryfall_id: "187fe54c-7d0c-4225-9d46-3affbead897d",
    ..TokenDef::DEFAULT
};

/// TMH3 #33: colorless 1/1 artifact Servo, with no rules abilities.
pub static SERVO_1_1: TokenDef = TokenDef {
    name: "Servo",
    types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
    subtypes: &[creature::SERVO],
    power: Some(1),
    toughness: Some(1),
    scryfall_id: "23165814-3eb8-4d94-b9c8-ecc798afe7a0",
    ..TokenDef::DEFAULT
};
/// TFRC #16: toxic is a result of combat damage, not a triggered ability.
pub static PHYREXIAN_MITE_1_1: TokenDef = TokenDef {
    name: "Phyrexian Mite",
    types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
    subtypes: &[creature::PHYREXIAN, creature::MITE],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::CANT_BLOCK,
    abilities: &[AbilityDef::Toxic { poison: 1 }],
    scryfall_id: "e9c68c64-f3b9-45b7-83d1-bd1439a49942",
    ..TokenDef::DEFAULT
};
/// TFDC #4: the black 0/0 Germ for living weapon.
pub static PHYREXIAN_GERM_0_0_BLACK: TokenDef = TokenDef {
    name: "Phyrexian Germ",
    colors: ColorSet::from_slice(&[Color::Black]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::PHYREXIAN, creature::GERM],
    power: Some(0),
    toughness: Some(0),
    scryfall_id: "53c05e53-84c7-4796-ae02-d778faa5afa6",
    ..TokenDef::DEFAULT
};
/// Serpent Generator's colorless artifact Snake. No matching printed token is
/// listed by Scryfall; the client draws its actual face rather than another Snake.
pub static SNAKE_1_1_ARTIFACT_POISON: TokenDef = TokenDef {
    name: "Snake",
    types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
    subtypes: &[creature::SNAKE],
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::This,
            player: baylee_cards_dsl::PlayerRel::EachPlayer
        },
        &[Effect::Poison {
            who: baylee_cards_dsl::PlayerRel::EventPlayer,
            amount: baylee_cards_dsl::Amount::Fixed(1)
        }]
    )],
    ..TokenDef::DEFAULT
};

/// Powerstone mana can pay any nonspell cost, or an artifact spell.
static POWERSTONE_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)
    .restricted_for(
        &Filter::ARTIFACT,
        baylee_cards_dsl::SpendRider::None,
        baylee_cards_dsl::ManaUse::NonSpellPaymentsOrMatchingSpells,
    ),])];

/// Colorless Powerstone artifact with restricted mana production.
pub static POWERSTONE: TokenDef = TokenDef {
    name: "Powerstone",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::POWERSTONE],
    abilities: POWERSTONE_MANA,
    scryfall_id: "d45fe4b6-aeaf-4f84-b660-c7b482ed8512",
    ..TokenDef::DEFAULT
};

/// Colorless Vibranium artifact with indestructible and Powerstone mana.
pub static VIBRANIUM: TokenDef = TokenDef {
    name: "Vibranium",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::VIBRANIUM],
    keywords: KeywordSet::INDESTRUCTIBLE,
    abilities: POWERSTONE_MANA,
    scryfall_id: "7f9e9b3a-c515-449a-95da-ee3f5175b74f",
    ..TokenDef::DEFAULT
};

/// Colorless Junk artifact, sacrificed at sorcery speed for a paid play this turn.
pub static JUNK: TokenDef = TokenDef {
    name: "Junk",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::JUNK],
    abilities: &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[Effect::ExileTopToPlay {
            who: baylee_cards_dsl::PlayerRel::You,
            play: baylee_cards_dsl::ExilePlay {
                count: 1,
                window: baylee_cards_dsl::PlayWindow::ThisTurn,
                free: false,
                mana_any_type: false,
                spell_types: None,
            },
        }],
        timing = baylee_cards_dsl::ActivationTiming::SorcerySpeed
    )],
    scryfall_id: "62c4a6f6-6425-4c0c-b35a-880fcab42aad",
    ..TokenDef::DEFAULT
};

/// Colorless Map artifact: a controlled creature explores at sorcery speed.
/// Scryfall TLCI #17, checked 2026-10-15; Wizards' LCI mechanics article
/// confirms the cost, target restriction and timing.
pub static MAP: TokenDef = TokenDef {
    name: "Map",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::MAP],
    abilities: &[activated!(
        cost!("{1}", TapSelf, SacrificeSelf),
        &[Effect::Explore {
            target: baylee_cards_dsl::TargetSpec::Object(&Filter::YOUR_CREATURE),
        }],
        target = Some(baylee_cards_dsl::TargetSpec::Object(&Filter::YOUR_CREATURE)),
        timing = baylee_cards_dsl::ActivationTiming::SorcerySpeed,
    )],
    scryfall_id: "64839118-09d2-4645-9d3c-f80755ac781f",
    ..TokenDef::DEFAULT
};

/// Ordinary colorless 1/1 Spirit, with no printed abilities.
pub static SPIRIT_1_1_COLORLESS: TokenDef = TokenDef {
    name: "Spirit",
    types: TypeSet::CREATURE,
    subtypes: &[creature::SPIRIT],
    power: Some(1),
    toughness: Some(1),
    scryfall_id: "c672ee51-0e77-463f-8586-191b413c44dd",
    ..TokenDef::DEFAULT
};

/// Colorless 1/1 Pilot whose bonus applies only to crew/saddle payments.
pub static PILOT_1_1_COLORLESS: TokenDef = TokenDef {
    name: "Pilot",
    types: TypeSet::CREATURE,
    subtypes: &[creature::PILOT],
    power: Some(1),
    toughness: Some(1),
    abilities: &[static_ability!(
        Filter::This,
        Modifier::CrewSaddlePowerBonus(2)
    )],
    scryfall_id: "8672d795-04f9-4089-9c92-6d6ff628da12",
    ..TokenDef::DEFAULT
};

/// Realm of Koh's colorless 1/1 Spirit can interact in combat only with Spirits.
pub static SPIRIT_1_1_COLORLESS_SPIRIT_COMBAT: TokenDef = TokenDef {
    name: "Spirit",
    types: TypeSet::CREATURE,
    subtypes: &[creature::SPIRIT],
    power: Some(1),
    toughness: Some(1),
    abilities: &[
        static_ability!(
            Filter::This,
            Modifier::CantBlock(&Filter::Not(&Filter::HasSubtype(creature::SPIRIT)))
        ),
        static_ability!(
            Filter::This,
            Modifier::CantBeBlockedBy(&Filter::Not(&Filter::HasSubtype(creature::SPIRIT)))
        ),
    ],
    scryfall_id: "f59eba51-458a-40e0-b754-999f91d5d839",
    ..TokenDef::DEFAULT
};

/// Recruit's 1/1 white Human Soldier (CR 701.70a), THOB token 2.
pub static HUMAN_SOLDIER_1_1_WHITE: TokenDef = TokenDef {
    name: "Human Soldier",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::HUMAN, creature::SOLDIER],
    power: Some(1),
    toughness: Some(1),
    scryfall_id: "6007af81-4541-4b55-90ea-03d365362ae5",
    ..TokenDef::DEFAULT
};

/// Empower Jace's blue planeswalker token (CR 701.71a).
pub static JACE_PLANESWALKER_BLUE: TokenDef = TokenDef {
    name: "Jace",
    colors: ColorSet::from_slice(&[Color::Blue]),
    types: TypeSet::PLANESWALKER,
    subtypes: &[baylee_core::generated::subtypes::planeswalker::JACE],
    loyalty: Some(0),
    defense: None,
    abilities: &[
        baylee_cards_dsl::loyalty!(-1, &[Effect::surveil(1)]),
        baylee_cards_dsl::loyalty!(-3, &[Effect::draw(1)]),
    ],
    ..TokenDef::DEFAULT
};

/// 2/2 black Zombie, M20 #6 — the ordinary token made by Field of the Dead
/// and by every later effect with the same body, not a card-specific variant.
pub static ZOMBIE_2_2_BLACK: TokenDef = TokenDef {
    name: "Zombie",
    colors: ColorSet::from_slice(&[Color::Black]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::ZOMBIE],
    power: Some(2),
    toughness: Some(2),
    scryfall_id: "18f0436e-9328-4266-9cf8-80b557a0c17c",
    ..TokenDef::DEFAULT
};

/// Every token there is; the index IS the stable token id (art key).
///
/// Append only — an insertion in the middle renumbers every token after it,
/// and the number is what a client has cached as an art key.
///
/// The table lives in [`crate::generated_tokens`] rather than here because
/// the tokens in it come from two places: the ones below, written by a person
/// for a card finished by hand, and the ones a reader wrote out of the
/// card-script reference. Both halves grow, so neither can be a block inside
/// the other — a hand-written token added at the end of *this* list would
/// have renumbered every generated one. What the ledger records instead is
/// the order ids were assigned in, whichever half an entry came from, and
/// `cargo xtask codegen` may only append to it.
pub use crate::generated_tokens::ALL;

/// The stable id of a central token (its index in [`ALL`]).
#[must_use]
pub fn token_id(token: &TokenDef) -> u16 {
    ALL.iter()
        .position(|t| std::ptr::eq(*t, token))
        .map_or(u16::MAX, |i| i as u16)
}

/// The token a stable id names, if it names one.
#[must_use]
pub fn by_token_id(id: u16) -> Option<&'static TokenDef> {
    ALL.get(id as usize).copied()
}

/// Verified printed ability text for central tokens, indexed by the DSL list.
/// Food: Scryfall ELD token 15, bf36408d-ed85-497f-8e68-d3a922c388a0,
/// checked 2026-10-02. Powerstone, Vibranium and Junk: the Scryfall records
/// on their definitions, checked 2026-10-11. Unknown pairs have no invented sentence.
#[must_use]
pub fn ability_text(id: u16, index: u32) -> Option<&'static str> {
    let token = by_token_id(id)?;
    if index < 2 && std::ptr::eq(token, &raw const SPIRIT_1_1_COLORLESS_SPIRIT_COMBAT) {
        return Some("This token can't block or be blocked by non-Spirit creatures.");
    }
    if index != 0 {
        return None;
    }
    if std::ptr::eq(token, &raw const PILOT_1_1_COLORLESS) {
        return Some(
            "This token saddles Mounts and crews Vehicles as though its power were 2 greater.",
        );
    }
    if std::ptr::eq(token, &raw const FOOD) {
        Some("{2}, {T}, Sacrifice this token: You gain 3 life.")
    } else if std::ptr::eq(token, &raw const POWERSTONE)
        || std::ptr::eq(token, &raw const VIBRANIUM)
    {
        Some("{T}: Add {C}. This mana can't be spent to cast a nonartifact spell.")
    } else if std::ptr::eq(token, &raw const MAP) {
        Some(
            "{1}, {T}, Sacrifice this artifact: Target creature you control explores. Activate only as a sorcery.",
        )
    } else if std::ptr::eq(token, &raw const JUNK) {
        Some(
            "{T}, Sacrifice this artifact: Exile the top card of your library. You may play that card this turn. Activate only as a sorcery.",
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests;

// Verified primary Scryfall cards/tlci/3 payload saved in gnome-primary.json.
static GNOME_SIZE: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::CharacteristicPT {
        count: PtCount::YouControl(&Filter::ARTIFACT_OR_CREATURE),
        toughness_plus: 0,
    }
)];
/// White artifact Gnome Soldier whose own CDA counts its controller's artifacts and creatures.
pub static GNOME_SOLDIER_WHITE_DYNAMIC: TokenDef = TokenDef {
    name: "Gnome Soldier",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
    subtypes: &[creature::GNOME, creature::SOLDIER],
    abilities: GNOME_SIZE,
    scryfall_id: "cbe7c36a-77b7-4418-9e2d-b6aeea61502e",
    ..TokenDef::DEFAULT
};

/// Invasion of Belenon's white and blue Knight with vigilance (MOM token 10).
pub static KNIGHT_2_2_WHITE_BLUE_VIGILANCE: TokenDef = TokenDef {
    name: "Knight",
    colors: ColorSet::from_slice(&[Color::White, Color::Blue]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::KNIGHT],
    power: Some(2),
    toughness: Some(2),
    keywords: KeywordSet::VIGILANCE,
    scryfall_id: "88439bfc-8942-473b-9e4f-863017788476",
    ..TokenDef::DEFAULT
};

static TETRAVITE_ABILITIES: &[AbilityDef] =
    &[static_ability!(Filter::This, Modifier::CantBeEnchanted)];

/// 1/1 colorless Tetravite artifact creature token with flying and
/// "This token can't be enchanted." (Tetravus).
pub static TETRAVITE_1_1_COLORLESS_ARTIFACT_FLYING: TokenDef = TokenDef {
    name: "Tetravite",
    types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
    subtypes: &[creature::TETRAVITE],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::FLYING,
    abilities: TETRAVITE_ABILITIES,
    ..TokenDef::DEFAULT
};

static WOLVES_OF_THE_HUNT_FILTER: Filter = Filter::And(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::Named("Wolves of the Hunt"),
]);

static WOLVES_OF_THE_HUNT_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::BandsWithOther(&WOLVES_OF_THE_HUNT_FILTER)
)];

/// 1/1 green Wolf creature token named Wolves of the Hunt with
/// "bands with other creatures named Wolves of the Hunt" (Master of the Hunt).
pub static WOLVES_OF_THE_HUNT_1_1_GREEN_WOLF: TokenDef = TokenDef {
    name: "Wolves of the Hunt",
    colors: ColorSet::from_slice(&[Color::Green]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::WOLF],
    power: Some(1),
    toughness: Some(1),
    abilities: WOLVES_OF_THE_HUNT_ABILITIES,
    ..TokenDef::DEFAULT
};

static HANWEIR_THE_WRITHING_TOWNSHIP_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::Attacks(&Filter::This),
    &[Effect::CreateTokenAttacking {
        token: &crate::generated_tokens::ELDRAZI_HORROR_3_2,
        amount: baylee_cards_dsl::Amount::Fixed(2),
        sacrifice_at_next_end_step: false,
    }]
)];

/// 7/4 colorless Legendary Creature — Eldrazi Ooze with trample, haste, and
/// "Whenever Hanweir, the Writhing Township attacks, create two 3/2 colorless
/// Eldrazi Horror creature tokens that are tapped and attacking." (Hanweir
/// Battlements // Hanweir Garrison meld back face, EMN #130b).
pub static HANWEIR_THE_WRITHING_TOWNSHIP: TokenDef = TokenDef {
    name: "Hanweir, the Writhing Township",
    colors: ColorSet::EMPTY,
    types: TypeSet::CREATURE,
    supertypes: baylee_core::types::SupertypeSet::LEGENDARY,
    subtypes: &[creature::ELDRAZI, creature::OOZE],
    power: Some(7),
    toughness: Some(4),
    keywords: KeywordSet::TRAMPLE.union(KeywordSet::HASTE),
    abilities: HANWEIR_THE_WRITHING_TOWNSHIP_ABILITIES,
    scryfall_id: "671fe14d-0070-4bc7-8983-707b570f4492",
    ..TokenDef::DEFAULT
};
