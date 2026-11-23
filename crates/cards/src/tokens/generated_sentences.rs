//! Printed rules for generated card tokens.
//!
//! The source of each row is the English Oracle of the card that creates the
//! token, or the predefined token's own rules. Ability positions follow the
//! compiled token, including Equip and Crew when represented as abilities.

use crate::generated_tokens as token;
use baylee_cards_dsl::TokenDef;

pub(super) static ROWS: &[(&TokenDef, &[&str])] = &[
    // Ajani, Strength of the Pride and Ajani, Resolute.
    (
        &token::AJANI_S_PRIDEMATE_2_2_WHITE_A5D7A6060,
        &["Whenever you gain life, put a +1/+1 counter on this token."],
    ),
    // Garruk, Cursed Huntsman.
    (
        &token::WOLF_2_2_BLACK_GREEN_A26E23A3D,
        &["When this token dies, put a loyalty counter on each Garruk you control."],
    ),
    // Everquill Phoenix.
    (
        &token::FEATHER_RED_A49C2DBA7,
        &[
            "{1}, Sacrifice this token: Return target Phoenix card from your graveyard to the battlefield tapped.",
        ],
    ),
    // Freyalise, Llanowar's Fury.
    (&token::ELF_DRUID_1_1_GREEN_A550D8680, &["{T}: Add {G}."]),
    // Roxanne, Starfall Savant.
    (
        &token::METEORITE_A68E96657,
        &[
            "When this token enters, it deals 2 damage to any target.",
            "{T}: Add one mana of any color.",
        ],
    ),
    // Commodore Guff.
    (
        &token::WIZARD_1_1_RED_AC89D6B73,
        &["{T}: Add {R}. Spend this mana only to cast a planeswalker spell."],
    ),
    // Fire Nation Attacks.
    (&token::SOLDIER_2_2_RED_FIREBENDING_1, &["Firebending 1"]),
    // The Legend of Roku // Avatar Roku.
    (
        &token::DRAGON_4_4_RED_FIREBENDING_4_FLYING,
        &["Firebending 4"],
    ),
    // There and Back Again.
    (
        &token::SMAUG_LEGENDARY_6_6_RED_FLYING_HASTE_AE2F21CCC,
        &["When Smaug dies, create fourteen Treasure tokens."],
    ),
    // Kaito Shizuki.
    (
        &token::NINJA_1_1_BLUE_A2CE7F4B9,
        &["This token can't be blocked."],
    ),
    // Rasputin, the Oneiromancer.
    (
        &token::KNIGHT_2_2_WHITE_PROTECTION_FROM_RED,
        &["Protection from red"],
    ),
    // Ajani Goldmane.
    (
        &token::AVATAR_X_X_WHITE_AA53C9B55,
        &["This token's power and toughness are each equal to your life total."],
    ),
    // Tezzeret the Schemer.
    (
        &token::ETHERIUM_CELL_A852C8CC6,
        &["{T}, Sacrifice this token: Add one mana of any color."],
    ),
    // Invasion of Kylem // Valor's Reach Tag Team.
    (
        &token::WARRIOR_3_2_WHITE_RED_A7C32C14A,
        &[
            "Whenever this token and at least one other creature token attack, put a +1/+1 counter on this token.",
        ],
    ),
    // Extus, Oriq Overlord // Awaken the Blood Avatar.
    (
        &token::AVATAR_3_6_BLACK_RED_HASTE_A5A48861B,
        &["Whenever this token attacks, it deals 3 damage to each opponent."],
    ),
    // Wrenn and Seven.
    (
        &token::TREEFOLK_X_X_GREEN_REACH_AC1FE3DC7,
        &["This token's power and toughness are each equal to the number of lands you control."],
    ),
    // Mesmerizing Benthid.
    (
        &token::ILLUSION_0_2_BLUE_A820721BF,
        &[
            "Whenever this token blocks a creature, that creature doesn't untap during its controller's next untap step.",
        ],
    ),
    // Niko Aris.
    (
        &token::SHARD_A4E3EA5DB,
        &["{2}, Sacrifice this token: Scry 1, then draw a card."],
    ),
    // Blacksmith's Talent. The compiled Equip ability precedes the stat bonus.
    (
        &token::SWORD_EQUIP_2_A837D70E1,
        &["Equip {2}", "Equipped creature gets +1/+1."],
    ),
    // White Tiger, Ava Ayala.
    (
        &token::THE_TIGER_GOD_LEGENDARY_4_4_GREEN_AE516C47A,
        &["The Tiger God can't be blocked by more than one creature."],
    ),
    // Minn, Wily Illusionist.
    (
        &token::ILLUSION_1_1_BLUE_AF5F23524,
        &["This token gets +1/+0 for each other Illusion you control."],
    ),
    // Serpent Generator.
    (
        &token::SNAKE_ARTIFACT_1_1,
        &["Whenever this creature deals damage to a player, that player gets a poison counter."],
    ),
    // Mysidian Elder.
    (
        &token::WIZARD_0_1_BLACK_AC10C40F2,
        &["Whenever you cast a noncreature spell, this token deals 1 damage to each opponent."],
    ),
    // Digsite Engineer.
    (
        &token::CONSTRUCT_ARTIFACT_0_0_A47644F8C,
        &["This token gets +1/+1 for each artifact you control."],
    ),
    // Basilica Shepherd.
    (
        &token::PHYREXIAN_MITE_ARTIFACT_1_1_TOXIC_1_A789D9A47,
        &["Toxic 1", "This token can't block."],
    ),
    // Back on Track.
    (
        &token::PILOT_1_1_A462BBD00,
        &["This token saddles Mounts and crews Vehicles as though its power were 2 greater."],
    ),
    // Lost in the Spirit World.
    (
        &token::SPIRIT_1_1_A22A1974D,
        &[
            "This token can't block non-Spirit creatures.",
            "This token can't be blocked by non-Spirit creatures.",
        ],
    ),
    // Junk token Oracle, as used for the predefined Junk.
    (
        &token::JUNK_ADF9DDBB8,
        &[
            "{T}, Sacrifice this artifact: Exile the top card of your library. You may play that card this turn. Activate only as a sorcery.",
        ],
    ),
    // Splintering Wind.
    (
        &token::SPLINTER_1_1_GREEN_CUMULATIVE_UPKEEP_G_FLYING,
        &["Cumulative upkeep {G}"],
    ),
    // Vernal Sovereign.
    (
        &token::ELEMENTAL_X_X_WHITE_GREEN_A89F4D877,
        &[
            "This token's power and toughness are each equal to the number of creatures you control.",
        ],
    ),
    // Thousand Moons Smithy.
    (
        &token::GNOME_SOLDIER_ARTIFACT_X_X_WHITE_A45F32D9B,
        &[
            "This token's power and toughness are each equal to the number of artifacts and/or creatures you control.",
        ],
    ),
    // Vibranium Mining Mech.
    (
        &token::VIBRANIUM_INDESTRUCTIBLE_AD775736C,
        &["{T}: Add {C}. This mana can't be spent to cast a nonartifact spell."],
    ),
    // Fable of the Mirror-Breaker.
    (
        &token::GOBLIN_SHAMAN_2_2_RED_AD6888B34,
        &["Whenever this token attacks, create a Treasure token."],
    ),
    // Cloudseeder.
    (
        &token::CLOUD_SPRITE_1_1_BLUE_FLYING_A92C7E842,
        &["This token can block only creatures with flying."],
    ),
    // Sound the Call.
    (
        &token::WOLF_1_1_GREEN_ABBBD5EBC,
        &["This token gets +1/+1 for each card named Sound the Call in each graveyard."],
    ),
    // Hunted Horror.
    (
        &token::CENTAUR_3_3_GREEN_PROTECTION_FROM_BLACK,
        &["Protection from black"],
    ),
    // Sparkspitter.
    (
        &token::SPARK_ELEMENTAL_3_1_RED_HASTE_TRAMPLE_A730DA31D,
        &["At the beginning of the end step, sacrifice this token."],
    ),
    // Elephant Resurgence.
    (
        &token::ELEPHANT_X_X_GREEN_A43AFFAD0,
        &[
            "This token's power and toughness are each equal to the number of creature cards in its controller's graveyard.",
        ],
    ),
    // Kalonian Twingrove.
    (
        &token::TREEFOLK_WARRIOR_X_X_GREEN_AB976E947,
        &["This token's power and toughness are each equal to the number of Forests you control."],
    ),
    // Goblin Kaboomist.
    (
        &token::LAND_MINE_A9765C567,
        &[
            "{R}, Sacrifice this token: This token deals 2 damage to target attacking creature without flying.",
        ],
    ),
    // Deathpact Angel.
    (
        &token::CLERIC_1_1_WHITE_BLACK_ABC71E4FA,
        &[
            "{3}{W}{B}{B}, {T}, Sacrifice this token: Return a card named Deathpact Angel from your graveyard to the battlefield.",
        ],
    ),
    // Rite of Belzenlok.
    (
        &token::DEMON_6_6_BLACK_FLYING_TRAMPLE_A2AEDFBDE,
        &[
            "At the beginning of your upkeep, sacrifice another creature. If you can't, this token deals 6 damage to you.",
        ],
    ),
    // Rekindling Phoenix.
    (
        &token::ELEMENTAL_0_1_RED_A8D932090,
        &[
            "At the beginning of your upkeep, sacrifice this token and return target card named Rekindling Phoenix from your graveyard to the battlefield. It gains haste until end of turn.",
        ],
    ),
    // Goblin Spymaster.
    (
        &token::GOBLIN_1_1_RED_AB8C78CBC,
        &["Creatures you control attack each combat if able."],
    ),
    // Nesting Dragon.
    (
        &token::DRAGON_EGG_0_2_RED_DEFENDER_A7327D884,
        &[
            "When this token dies, create a 2/2 red Dragon creature token with flying and '{R}: This token gets +1/+0 until end of turn.'",
        ],
    ),
    // Relic Robber.
    (
        &token::GOBLIN_CONSTRUCT_ARTIFACT_0_1_A051E5D11,
        &[
            "This token can't block.",
            "At the beginning of your upkeep, this token deals 1 damage to you.",
        ],
    ),
    // Doomed Artisan.
    (
        &token::SCULPTURE_ARTIFACT_X_X_AADDD2C1B,
        &[
            "This token's power and toughness are each equal to the number of Sculptures you control.",
        ],
    ),
    // Wolf's Quarry.
    (
        &token::BOAR_1_1_GREEN_A324A14E3,
        &["When this token dies, create a Food token."],
    ),
    // Toggo, Goblin Weaponsmith.
    (
        &token::ROCK_EQUIP_1_AF5CF245F,
        &[
            "Equip {1}",
            "Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target.'",
        ],
    ),
    // Kibo, Uktabi Prince.
    (
        &token::BANANA_A64649F03,
        &["{T}, Sacrifice this token: Add {R} or {G}. You gain 2 life."],
    ),
    // Pink Horror.
    (
        &token::BLUE_HORROR_2_2_BLUE_RED_AEA5841D5,
        &[
            "Whenever you cast an instant or sorcery spell, this token deals 1 damage to any target.",
        ],
    ),
    // Careful Cultivation.
    (&token::HUMAN_MONK_1_1_GREEN_A550D8680, &["{T}: Add {G}."]),
    // Icingdeath, Frost Tyrant.
    (
        &token::ICINGDEATH_FROST_TONGUE_LEGENDARY_ARTIFACT_WHITE_EQUIP_2_A815D4D79,
        &[
            "Equip {2}",
            "Equipped creature gets +2/+0.",
            "Whenever equipped creature attacks, tap target creature defending player controls.",
        ],
    ),
    // Expendable Lackey.
    (
        &token::FISH_1_1_BLUE_A2CE7F4B9,
        &["This token can't be blocked."],
    ),
    // Hallowed Haunting.
    (
        &token::SPIRIT_CLERIC_X_X_WHITE_A0BB72D6F,
        &["This token's power and toughness are each equal to the number of Spirits you control."],
    ),
    // Lita, Mechanical Engineer.
    (&token::ZEPPELIN_5_5_CREW_3_FLYING, &["Crew 3"]),
    // Synapse Necromage.
    (
        &token::FUNGUS_1_1_BLACK_AA6F088A4,
        &["This token can't block."],
    ),
    // Harried Spearguard.
    (
        &token::RAT_1_1_BLACK_AA6F088A4,
        &["This token can't block."],
    ),
    // Wake the Dragon.
    (
        &token::DRAGON_6_6_BLACK_RED_FLYING_MENACE_A4D1010B5,
        &[
            "Whenever this token deals combat damage to a player, gain control of target artifact that player controls.",
        ],
    ),
    // Mabel, Heir to Cragflame.
    (
        &token::CRAGFLAME_LEGENDARY_ARTIFACT_EQUIP_2_ABC95D015,
        &[
            "Equip {2}",
            "Equipped creature gets +1/+1.",
            "Equipped creature has vigilance, trample, and haste.",
        ],
    ),
    // Form a Posse.
    (
        &token::MERCENARY_1_1_RED_A7F82B7A6,
        &[
            "{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.",
        ],
    ),
    // Satyr's Cunning.
    (
        &token::SATYR_1_1_RED_AA6F088A4,
        &["This token can't block."],
    ),
    // Into the Fae Court.
    (
        &token::FAERIE_1_1_BLUE_FLYING_A92C7E842,
        &["This token can block only creatures with flying."],
    ),
    // Gwaihir, Greatest of the Eagles.
    (
        &token::BIRD_3_3_WHITE_FLYING_AB0E93B9C,
        &["Whenever this token attacks, target attacking creature gains flying until end of turn."],
    ),
    // Lluwen, Exchange Student.
    (
        &token::PEST_1_1_BLACK_GREEN_A2D920923,
        &["Whenever this token attacks, you gain 1 life."],
    ),
    // Station Monitor.
    (
        &token::DRONE_ARTIFACT_1_1_FLYING_A92C7E842,
        &["This token can block only creatures with flying."],
    ),
    // Wish Good Luck.
    (&token::VEHICLE_3_2_CREW_1, &["Crew 1"]),
    // Sami's Curiosity.
    (
        &token::LANDER_A23CC7A72,
        &[
            "{2}, {T}, Sacrifice this token: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
        ],
    ),
    // Ooze Spill.
    (
        &token::MUTAGEN_AD389F0F9,
        &[
            "{1}, {T}, Sacrifice this token: Put a +1/+1 counter on target creature. Activate only as a sorcery.",
        ],
    ),
    // Ozox, the Clattering King.
    (
        &token::JUMBLEBONES_LEGENDARY_2_1_BLACK_A0376465E,
        &[
            "Jumblebones can't block.",
            "When Jumblebones leaves the battlefield, return target card named Ozox, the Clattering King from your graveyard to your hand.",
        ],
    ),
    // Weapons Manufacturing.
    (
        &token::MUNITIONS_A90AA358F,
        &["When this token leaves the battlefield, it deals 2 damage to any target."],
    ),
    // Dragonbroods' Relic.
    (
        &token::RELIQUARY_DRAGON_4_4_WHITE_BLUE_BLACK_RED_GREEN_FLYING_LIFELINK_A80118C78,
        &["When this token enters, it deals 3 damage to any target."],
    ),
    // Setzer, Wandering Gambler.
    (
        &token::THE_BLACKJACK_LEGENDARY_ARTIFACT_3_3_CREW_2_FLYING,
        &["Crew 2"],
    ),
    // Iron Hills Blacksmith.
    (
        &token::AXE_EQUIP_2_AFDFCFBD2,
        &["Equip {2}", "Equipped creature gets +1/+0."],
    ),
    // Vraska, Soul of Stone.
    (
        &token::SCULPTURE_TREASURE_ARTIFACT_1_1_A852C8CC6,
        &["{T}, Sacrifice this token: Add one mana of any color."],
    ),
    // The Coming of Galactus.
    (
        &token::GALACTUS_LEGENDARY_16_16_BLACK_FLYING_TRAMPLE_A8C6751BA,
        &["Whenever Galactus attacks, destroy target land."],
    ),
    // Kwia, Vigorbloom.
    (
        &token::LOTUS_A48F41BDD,
        &["{T}, Sacrifice this token: Add three mana of any one color."],
    ),
    // Stable Wormhole.
    (
        &token::NEW_PLANET_A20C146E5,
        &["{T}: Add one mana of any color."],
    ),
    // Pursued Whale.
    (
        &token::PIRATE_1_1_RED_A9DD811C5,
        &[
            "This token can't block.",
            "Creatures you control attack each combat if able.",
        ],
    ),
    // Konstrari Improviser.
    (
        &token::HEARTWOOD_RED_GREEN_A6AEC5730,
        &["{T}: Add {R} or {G}."],
    ),
    // Origin of Captain America.
    (
        &token::STURDY_SHIELD_EQUIP_2_AEBF22228,
        &["Equip {2}", "Equipped creature gets +1/+2."],
    ),
    // The Sentry, Golden Guardian.
    (
        &token::THE_VOID_LEGENDARY_5_5_BLACK_FLYING_INDESTRUCTIBLE_AFB98BE81,
        &["The Void attacks each combat if able."],
    ),
    // Ka-Zar of the Savage Land.
    (
        &token::ZABU_LEGENDARY_2_2_GREEN_A7B54B8C1,
        &["Landfall — Whenever a land you control enters, put a +1/+1 counter on Zabu."],
    ),
    // Cleveland Booker, Empath.
    (
        &token::WORM_2_2_RED_GREEN_AA6F088A4,
        &["This token can't block."],
    ),
    // Tribble Cluster.
    (
        &token::TRIBBLE_1_1_WHITE_AA6F088A4,
        &["This token can't block."],
    ),
    // Vren, the Relentless.
    (
        &token::RAT_1_1_BLACK_AB5894BA6,
        &["This token gets +1/+1 for each other Rat you control."],
    ),
    // Cybernetica Datasmith.
    (
        &token::ROBOT_ARTIFACT_4_4_AA6F088A4,
        &["This token can't block."],
    ),
    // Estrid, the Masked.
    (
        &token::MASK_WHITE_ENCHANT_PERMANENT_UMBRA_ARMOR,
        &["Enchant permanent"],
    ),
    // Cursed Role (Wilds of Eldraine).
    (
        &token::CURSED_ENCHANT_CREATURE_A10F098BB,
        &[
            "Enchant creature",
            "Enchanted creature has base power and toughness 1/1.",
        ],
    ),
    // Monster Role (Wilds of Eldraine).
    (
        &token::MONSTER_ENCHANT_CREATURE_AD419DDA4,
        &[
            "Enchant creature",
            "Enchanted creature gets +1/+1.",
            "Enchanted creature has trample.",
        ],
    ),
    // Royal Role (Wilds of Eldraine).
    (
        &token::ROYAL_ENCHANT_CREATURE_AAB276179,
        &[
            "Enchant creature",
            "Enchanted creature gets +1/+1.",
            "Enchanted creature has ward {1}.",
        ],
    ),
    // Sorcerer Role (Wilds of Eldraine).
    (
        &token::SORCERER_ENCHANT_CREATURE_A77120B1D,
        &[
            "Enchant creature",
            "Enchanted creature gets +1/+1.",
            "Enchanted creature has \"Whenever this creature attacks, scry 1.\"",
        ],
    ),
    // Virtuous Role (Wilds of Eldraine).
    (
        &token::VIRTUOUS_ENCHANT_CREATURE_AABA4C452,
        &[
            "Enchant creature",
            "Enchanted creature gets +1/+1 for each enchantment you control.",
        ],
    ),
    // Wicked Role (Wilds of Eldraine).
    (
        &token::WICKED_ENCHANT_CREATURE_AAA10B008,
        &[
            "Enchant creature",
            "Enchanted creature gets +1/+1.",
            "When this token is put into a graveyard, each opponent loses 1 life.",
        ],
    ),
    // Young Hero Role (Wilds of Eldraine).
    (
        &token::YOUNG_HERO_ENCHANT_CREATURE_A6CE89E44,
        &[
            "Enchant creature",
            "Enchanted creature has \"Whenever this creature attacks, if its toughness is 3 or less, put a +1/+1 counter on it.\"",
        ],
    ),
    // Preston Garvey, Minuteman.
    (
        &token::SETTLEMENT_GREEN_ENCHANT_LAND_A9179052F,
        &[
            "Enchant land",
            "Enchanted land has \"{T}: Add one mana of any color.\"",
        ],
    ),
    // The Prydwen, Steel Flagship.
    (
        &token::HUMAN_KNIGHT_2_2_WHITE_A94B73029,
        &[
            "This token gets +2/+2 as long as an artifact entered the battlefield under your control this turn.",
        ],
    ),
];
