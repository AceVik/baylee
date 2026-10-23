//! Exact activation-reader census over the compiled card pool.

use super::*;

/// How many of the pool's activated and loyalty abilities the table reads,
/// as `(read, of)`, on an empty board.
///
/// An unread ability is one the house leaves alone (a deep profile's
/// loyalty falls back to `effect_value`). The floor catches a row lost; the
/// ceiling catches a wildcard that would read everything as worth
/// something, which is the one change that would make every unread ability
/// a guess.
#[test]
fn the_table_reads_most_of_what_the_pool_activates() {
    let v = view(0, &[20, 20], vec![]);
    let origin = Origin {
        id: obj(1),
        object: None,
    };
    let (mut read, mut of) = (0usize, 0usize);
    for def in baylee_cards::all() {
        for face in 0..def.faces.len() {
            for ability in def.abilities_for_face(face) {
                let effects = match ability {
                    AbilityDef::ModalActivated {
                        mana_ability: false,
                        ..
                    } => {
                        // Activation valuation refuses unbound mode semantics.
                        of += 1;
                        continue;
                    }
                    AbilityDef::Activated {
                        effects,
                        mana_ability: false,
                        ..
                    }
                    | AbilityDef::ActivatedConditional {
                        effects,
                        mana_ability: false,
                        ..
                    }
                    | AbilityDef::Loyalty { effects, .. } => *effects,
                    _ => continue,
                };
                of += 1;
                if agent()
                    .effects_worth(&v, origin, effects, Aim::Source, 0)
                    .is_some()
                {
                    read += 1;
                }
            }
        }
    }
    // Exact census after Batch47: newly implemented public activations include
    // command-zone entry, regeneration prevention, counter removal and
    // discard-LKI conditions. The table deliberately leaves these contextual
    // effects unread; raising their value without modelling them is a guess.
    // The former whole-percent floor rounded 1027/1286 down to 79, hiding that
    // this is pool growth rather than a lost reader row. Pin both populations
    // so additions and reader regressions require an explicit review.
    // Batch48 adds Adanto’s token activation: one new, fully read row.
    // Unstable Frontier adds a land-type choice whose contextual value the
    // table does not yet infer; it adds one activation, not one readable row.
    // Mystifying Maze adds a contextual delayed blink activation.
    // The One Ring adds a contextual counter-count draw activation.
    // Quarum Trench Gnomes adds a contextual permanent mana replacement.
    // Rebased main adds Goblin Artisans: one activation with a readable outcome.
    // Maze's End's Gate tutor now ends in "if you control ten or more Gates
    // with different names, you win the game" (astra-review): the table reads
    // neither that condition nor `WinGame`, so the row is unread, not guessed.
    // The wave 2 partials (pairs C and D) add nine activations and two read
    // rows: Gold-Forge Garrison's Golem and Grist's −5 are read; Gauntlets of
    // Chaos, Golden Guardian's fight, Grist's −2, Mirror Universe, Mount Doom,
    // Path of Mettle and The Black Gate are contextual.
    // Wave 3 of the partials lane (pairs E and F, 2026-10-13) adds four
    // activations and two read rows on top. Wave 4's Aladdin's Lamp adds one
    // activation whose worth (a better next draw) the table does not read.
    // One census over the whole pool (15.10.2026). Until then it was split
    // into the Generated cards and the rest, but a card test moves a card
    // from one to the other (codegen promotes it), so each test landing
    // broke both pins without any reader change. The total does not move
    // when a card is promoted and still moves for every row a reader gains
    // or loses: the full corpus brought it to 2999 activations, 2521 read
    // (the old split was 1034/1311 + 1487/1688).
    // Batch63 adds Amonkhet Raceway's haste activation (PumpTarget) and
    // Avishkar Raceway's draw activation: both use existing readable effects.
    // Muraganda Raceway adds a mana ability, excluded from this census, and
    // StartYourEngines is a synthesized keyword rather than an activation.
    // Resumed Batch55 adds Secret Tunnel's paired-target activation and
    // Fire Nation Palace's firebending grant: two new unread rows (the
    // evaluator lacks the second target binding and triggered grant value).
    // Stardew Valley's existing draw activation loses its readable row when
    // its chosen-player optional control transfer is completed. The exact
    // chooser/reference context is unavailable, so it is deliberately unknown.
    // Arena of Glory and Chocobo Camp change only mana abilities, excluded here.
    // Resumed batches 51/57/68 add four contextual nonmana activations:
    // Nova Pentacle and Shimian Night Stalker redirect future damage,
    // Temple of Power transforms its source, and Diamond City moves a
    // shield counter. None has an empty-board valuation. Cuombajj Witches
    // loses its previously readable first-damage-only row now that both
    // independently chosen target instances are represented faithfully.
    // Batch60 adds one modal activation each for Pyramids and Triassic Egg.
    // Their unbound modes remain unread; Egg's existing hatchling activation
    // is unchanged. Count both new rows without inventing a combined value.
    // Batch53 adds 30 activated and loyalty abilities across planeswalkers
    // (The Theorist, Jace Beleren; Nissa, Vital Force; Professor Dellian Fel;
    // Sivitri, Dragon Master; Tamiyo, Seasoned Scholar), equipment/classes
    // (Shadowspear, Umezawa's Jitte, Innkeeper's Talent), Dark Depths,
    // Vampire Hexmage, Faerie Mastermind, Tony Stark, and Psychic Frog, of
    // which 17 are read by the evaluator.
    // The 24-card Partial-to-Implemented completion batch adds 12 contextual
    // non-mana activated abilities (Mycoid Maze, Tomb of the Dusk Rose,
    // Northampton Farm, Hostile Hostel, Tetravus, Shelkin Brownie, Tolaria,
    // Master of the Hunt, Underdark Rift, Golgothian Sylex, Tawnos's Coffin,
    // Hanweir Battlements, Dungeon Descent).
    // The 67-card Fallen Empires (`fem`) completion batch adds 44 non-mana
    // activated abilities across spore/thallid saproling generators, storage
    // lands, pump/prevention/counter-manipulation artifacts and creatures, of
    // which 20 are read by the evaluator.
    // Ice Age (`ice`) Batch 1 adds 11 non-mana activated abilities across
    // Aegis of the Meek, Arcum's Sleigh, Arcum's Whistle, Arnjlot's Ascent,
    // Ashen Ghoul, Balduvian Hydra, Balduvian Shaman, Barbarian Guides,
    // Blinking Spirit, and Bone Shaman, of which 8 are read by the evaluator.
    // Ice Age (`ice`) Batch 2 adds 7 non-mana activated abilities across
    // Brown Ouphe, Celestial Sword, Crown of the Ages, Dreams of the Dead,
    // Dwarven Armory, Elkin Bottle, and Elvish Healer, of which 3 are read by
    // the evaluator.
    // Ice Age (`ice`) Batch 3 adds 6 non-mana activated abilities across
    // Flow of Maggots, Foul Familiar, Freyalise Supplicant, Freyalise's Charm,
    // and Fylgja, of which 4 are read by the evaluator.
    // Ice Age (`ice`) Batch 4 adds 13 non-mana activated abilities across
    // Gangrenous Zombies, General Jarkeld, Giant Trap Door Spider, Goblin Lyre,
    // Goblin Sappers, Goblin Ski Patrol, Goblin Snowman, Hallowed Ground,
    // Hecatomb, Hot Springs, Ice Cauldron, and Iceberg, of which 7 are read by
    // the evaluator.
    // Ice Age (`ice`) Batch 5 adds 14 non-mana activated abilities across
    // Infernal Denizen, Infinite Hourglass, Jester's Cap, Jester's Mask,
    // Jeweled Amulet, Karplusan Yeti, Kjeldoran Dead, Kjeldoran Elite Guard,
    // Kjeldoran Guard, Kjeldoran Royal Guard, Krovikan Elementalist, and
    // Krovikan Sorcerer, of which 6 are read by the evaluator.
    // Ice Age (`ice`) Batch 6 adds 8 non-mana activated abilities across
    // Legions of Lim-Dûl, Magus of the Unseen, Mercenaries, Merieke Ri Berit,
    // Mesmeric Trance, Minion of Leshrac, Mistfolk, and Mole Worms, of which
    // 4 are read by the evaluator.
    // Ice Age (`ice`) Batch 7 adds 7 non-mana activated abilities across
    // Musician, Norritt (2), Oath of Lim-Dûl, Orcish Farmer, and Phantasmal
    // Mount, of which 3 are read by the evaluator.
    // Ice Age (`ice`) Batch 8 adds 7 non-mana activated abilities across
    // Runed Arch, Skeleton Ship, Snow Fortress (3), Snow Hound, and Soldevi
    // Simulacrum, all 7 of which are read by the evaluator.
    // Ice Age (`ice`) Batch 9 adds 7 non-mana activated abilities across
    // Stormbind, Tinder Wall, Urza's Bauble, Vexing Arcanix, Walking Wall,
    // Whiteout, and Withering Wisps, of which 3 are read by the evaluator.
    // Homelands (`hml`) Batch 1 adds 15 non-mana activated abilities across
    // Aether Storm, Apocalypse Chime, Autumn Willow, Baron Sengir, Chandler,
    // Clockwork Gnomes, Clockwork Steed, Clockwork Swarm, Coral Reef (2), Dark
    // Maze, Daughter of Autumn, Didgeridoo, Dwarven Sea Clan, and Faerie Noble,
    // of which 12 are read by the evaluator.
    // Homelands (`hml`) Batch 2 adds 7 non-mana activated abilities across
    // Giant Oyster, Hazduhr the Abbot, Heart Wolf, Joven's Tools, Marjhan (2),
    // and Reveka, Wizard Savant, of which 2 are read by the evaluator.
    // Homelands (`hml`) Batch 3 adds 7 non-mana activated abilities across
    // Samite Alchemist, Sea Troll, Serrated Arrows, Torture, Trade Caravan,
    // and Willow Priestess (2), of which 3 are read by the evaluator.
    // Alliances (`all`) Batch 1 adds 4 non-mana activated abilities across
    // Ashnod's Cylix, Balduvian Dead, Browse, and Chaos Harlequin, of which 3
    // are read by the evaluator.
    // Alliances (`all`) Batch 2 adds 10 non-mana activated abilities across
    // Floodwater Dam, Gargantuan Gorilla, Gorilla Shaman, Gustha's Scepter (2),
    // Helm of Obedience, Ivory Gargoyle, Kjeldoran Pride, Krovikan Horror, and
    // Krovikan Plague, of which 7 are read by the evaluator.
    // Alliances (`all`) Batch 3 adds 12 non-mana activated abilities across
    // Lodestone Bauble, Lord of Tresserhorn, Mystic Compass, Nature's Blessing,
    // Phantasmal Fiend (2), Phelddagrif (3), Phyrexian Devourer, Phyrexian Portal,
    // Royal Herbalist, and Scarab of the Unseen, of which 4 are read by the evaluator.
    // Alliances (`all`) Batch 4 adds 15 non-mana activated abilities across
    // Seasoned Tactician, Soldevi Digger, Soldevi Heretic, Soldevi Sage, Soldevi
    // Sentry, Soldier of Fortune, Spiny Starfish, Splintering Wind, Storm
    // Elemental (2), Sworn Defender, Thought Lash, Tidal Control (2), and Tornado,
    // of which 7 are read by the evaluator.
    // Alliances (`all`) Batch 5 adds 11 non-mana activated abilities across
    // Unlikely Alliance, Urza's Engine (2), Varchild's Crusader, Veteran's Voice,
    // Viscerid Armor, Wandering Mage (3), Whip Vine, and Whirling Catapult, of
    // which 5 are read by the evaluator.
    // Mirage (`mir`) Batch 1 adds 6 non-mana activated abilities across
    // Abyssal Hunter, Acidic Dagger, Amber Prison, Amulet of Unmaking,
    // Barbed-Back Wurm, and Bone Mask, of which 1 is read by the evaluator.
    // Mirage (`mir`) Batch 2 adds 11 non-mana activated abilities across
    // Burning Palm Efreet, Burning Shield Askari, Cadaverous Knight, Canopy Dragon,
    // Chariot of the Sun, Circle of Despair, Civic Guildmage (2), Crimson Hellkite,
    // Cycle of Life, and Daring Apprentice, of which 5 are read by the evaluator.
    // Mirage (`mir`) Batch 3 adds 6 non-mana activated abilities across
    // Energy Vortex, Ersatz Gnomes (2), Ethereal Champion, Femeref Knight, and
    // Flame Elemental, of which 4 are read by the evaluator.
    // Mirage (`mir`) Batch 4 adds 8 non-mana activated abilities across
    // Frenetic Efreet, Goblin Soothsayer, Goblin Tinkerer, Grinning Totem,
    // Hakim, Loreweaver (2), Hammer of Bogardan, and Hivis of the Scale, of
    // which 3 are read by the evaluator.
    // Mirage (`mir`) Batch 5 adds 6 non-mana activated abilities across
    // Leering Gargoyle, Mangara's Tome, Mire Shade, Mist Dragon (2), and
    // Mtenda Griffin, of which 3 are read by the evaluator.
    // Mirage (`mir`) Batch 6 adds 5 non-mana activated abilities across
    // Prismatic Circle, Pyric Salamander, Raging Spirit, Rashida Scalebane, and
    // Reckless Embermage, of which 2 are read by the evaluator.
    // Mirage (`mir`) Batch 7 adds 6 non-mana activated abilities across
    // Sawback Manticore (2), Searing Spear Askari, Shadow Guildmage (2), and
    // Shauku, Endbringer, of which 5 are read by the evaluator.
    // Mirage (`mir`) Batch 8 adds 10 non-mana activated abilities across
    // Spatial Binding, Suq'Ata Firewalker, Tainted Specter, Telim'Tor's Darts,
    // Unerring Sling, Urborg Panther (2), Ventifact Bottle, and Vigilant Martyr
    // (2), of which 8 are read by the evaluator.
    // Mirage (`mir`) Batch 9 adds 5 non-mana activated abilities across
    // Wall of Corpses, Wave Elemental, Zhalfirin Commander, Zhalfirin Knight,
    // and Zirilan of the Claw, all 5 of which are read by the evaluator.
    // Visions (`vis`) Batch 1 adds 4 non-mana activated abilities across
    // Brass-Talon Chimera, Crypt Rats, Daraja Griffin, and Dragon Mask, of
    // which 3 are read by the evaluator.
    // Visions (`vis`) Batch 2 adds 8 non-mana activated abilities across
    // Flooded Shoreline, Giant Caterpillar, Gossamer Chains, Iron-Heart
    // Chimera, Juju Bubble, Keeper of Kookus, Knight of Valor, and Kookus, of
    // which 6 are read by the evaluator.
    // Visions (`vis`) Batch 3 adds 6 non-mana activated abilities across
    // Lead-Belly Chimera, Matopi Golem, Mundungu, Rainbow Efreet, Resistance
    // Fighter, and Righteous Aura, of which 3 are read by the evaluator.
    // Visions (`vis`) Batch 4 adds 6 non-mana activated abilities across
    // Snake Basket, Sun Clasp, Teferi's Honor Guard, Tin-Wing Chimera,
    // Triangle of War, and Vanishing, of which 2 are read by the evaluator.
    // Visions (`vis`) Batch 5 adds 1 non-mana activated ability on Wand of
    // Denial, which is not read by the evaluator.
    // Weatherlight (`wth`) Batch 1 adds 8 non-mana activated abilities across
    // Alms, Aura of Silence, Avizoa, Benalish Missionary, Betrothed of Fire
    // (2), Bösium Strip, and Call of the Wild, of which 6 are read by the
    // evaluator.
    // Weatherlight (`wth`) Batch 2 adds 9 non-mana activated abilities across
    // Chimeric Sphere (2), Coils of the Medusa, Downdraft (2), Dwarven
    // Thaumaturgist, Ertai's Familiar, Fire Whip, and Fungus Elemental, of
    // which 2 are read by the evaluator.
    // Weatherlight (`wth`) Batch 3 adds 7 non-mana activated abilities across
    // Goblin Bomb, Jabari's Banner, Kithkin Armor, Llanowar Druid, Master of
    // Arms, Nature's Kiss, and Necratog, of which 3 are read by the evaluator.
    // Weatherlight (`wth`) Batch 4 adds 6 non-mana activated abilities across
    // Orcish Settlers, Phantom Wings, Phyrexian Furnace (2), Serrated
    // Biskelion, and Thran Tome, of which 5 are read by the evaluator.
    // Weatherlight (`wth`) Batch 5 adds 5 non-mana activated abilities across
    // Touchstone, Tranquil Grove, Vodalian Illusionist, Well of Knowledge, and
    // Zombie Scavengers, of which 3 are read by the evaluator.
    // Tempest (`tmp`) Batch 1 adds 13 non-mana activated abilities across Altar
    // of Dementia, Bounty Hunter (2), Canyon Drake, Carrionette, Circle of
    // Protection: Shadow, Coffin Queen, Cold Storage (2), Crazed Armodon, Crown
    // of Flames (2), and Cursed Scroll, of which 8 are read before the two
    // generated cards promote to Implemented.
    // Tempest (`tmp`) Batch 2 adds 7 non-mana activated abilities across
    // Dracoplasm, Echo Chamber, Enraging Licid, Essence Bottle (2), Excavator,
    // and Fevered Convulsions, of which 5 are read by the evaluator.
    // Tempest (`tmp`) Batch 3 adds 12 non-mana activated abilities across
    // Flickering Ward, Flowstone Salamander, Flowstone Sculpture, Fool's Tome,
    // Gerrard's Battle Cry, Grindstone, Heartwood Giant, Helm of Possession,
    // Jinxed Idol, Knight of Dawn, Knight of Dusk, and Leeching Licid, of
    // which 10 are read by the evaluator.
    // Tempest (`tmp`) Batch 4 adds 12 non-mana activated abilities across
    // Legacy's Allure, Maddening Imp, Magnetic Web,Mounted Archers, Nurturing
    // Licid, Opportunist, Oracle en-Vec, Phyrexian Grimoire, and Phyrexian
    // Splicer (4), of which 3 are read by the evaluator.
    // Tempest (`tmp`) Batch 5 adds 5 non-mana activated abilities across
    // Quickening Licid, Rootwater Matriarch, Sacred Guide, Safeguard, and
    // Scroll Rack, of which 3 are read by the evaluator.
    // Tempest (`tmp`) Batch 6 adds 10 non-mana activated abilities across
    // Selenia, Dark Angel, Shimmering Wings, Skyshroud Ranger, Starke of Rath,
    // Stinging Licid, Thalakos Mistfolk, Tooth and Claw, Torture Chamber,
    // Tradewind Rider, and Trumpeting Armodon, of which 5 are read by the
    // evaluator.
    // Tempest (`tmp`) Batch 7 adds 3 non-mana activated abilities across
    // Vhati il-Dal, Volrath's Curse, and Wood Sage, of which 1 is read by the
    // evaluator.
    // Stronghold (`sth`) Batch 1 adds 8 non-mana activated abilities across
    // Amok, Bullwhip, Calming Licid, Conviction, Convulsing Licid, Corrupting
    // Licid, Crovax the Cursed, and Duct Crawler, of which 4 are read by the
    // evaluator.
    // Stronghold (`sth`) Batch 2 adds 4 non-mana activated abilities across
    // Gliding Licid, Hornet Cannon, Jinxed Ring, and Mindwarper, of which 1 is
    // read by the evaluator.
    // Stronghold (`sth`) Batch 3 adds 6 non-mana activated abilities across
    // Pursuit of Knowledge, Samite Blessing, Shard Phoenix (2), Silver Wyvern,
    // Skeleton Scavengers, and Tempting Licid, of which 3 are read by the
    // evaluator.
    // Stronghold (`sth`) Batch 4 adds 3 non-mana activated abilities across
    // Volrath's Gardens, Volrath's Laboratory, and Volrath's Shapeshifter, of
    // which 2 are read by the evaluator.
    // Exodus (`exo`) Batch 1 adds 6 non-mana activated abilities across Cat
    // Burglar, Cinder Crawler, Dauthi Cutthroat, Dauthi Jackal, Dizzying Gaze,
    // and Ephemeron, of which 5 are read by the evaluator.
    // Exodus (`exo`) Batch 2 adds 8 non-mana activated abilities across Keeper
    // of the Beasts, Keeper of the Dead, Keeper of the Flame, Keeper of the
    // Light, Keeper of the Mind, Mage il-Vec, Mogg Assassin, and Null Brooch,
    // all 8 of which are read by the evaluator.
    // Exodus (`exo`) Batch 3 adds 7 non-mana activated abilities across Ogre
    // Shaman, Plaguebearer, Pygmy Troll, Reconnaissance, Rootwater Mystic,
    // Shackles, and Skyshaper, of which 5 are read by the evaluator.
    // Exodus (`exo`) Batch 4 adds 7 non-mana activated abilities across Spike
    // Rogue, Thalakos Scout, Thopter Squadron (two), Thrull Surgeon, Volrath's
    // Dungeon, and Wayward Soul, of which 6 are read by the evaluator.
    // Urza's Saga (`usg`) Batch 1 adds 7 non-mana activated abilities across
    // Albino Troll, Argothian Elder, Barrin's Codex, Brand (cycling), Carrion
    // Beetles, Chimeric Staff, and Citanul Flute, of which 4 are read by the
    // evaluator.
    // Urza's Saga (`usg`) Batch 2 adds 7 non-mana activated abilities across
    // Copper Gnomes, Crystal Chimes, Discordant Dirge, Endoskeleton, Faith
    // Healer, Fiery Mantle, and Fire Ants, all 7 of which are read by the
    // evaluator.
    // Urza's Saga (`usg`) Batch 3 adds 2 non-mana activated abilities across
    // Fortitude and Greater Good, of which 1 is read by the evaluator.
    // Urza's Saga (`usg`) Batch 4 adds 10 non-mana activated abilities across
    // Karn, Silver Golem, Lilting Refrain, Lurking Evil, Mana Leech,
    // Metrognome, Midsummer Revel, Mishra's Helix, Mobile Fort, No Rest for
    // the Wicked, and Opal Acrolith, of which 4 are read by the evaluator.
    // Urza's Saga (`usg`) Batch 5 adds 9 non-mana activated abilities across
    // Phyrexian Colossus, Phyrexian Processor, Recantation, Rumbling Crescendo,
    // Rune of Protection: Artifacts, Sanctum Guardian, Serra's Hymn, and
    // Serra's Liturgy, of which 6 are read by the evaluator.
    // Urza's Saga (`usg`) Batch 6–7, Urza's Legacy (`ulg`) Batches 1–4, and
    // Urza's Destiny (`uds`) Batch 1 add 48 non-mana activated abilities, of
    // which 24 are read by the evaluator.
    // The review of those batches took 36 abilities off cards that
    // played more than they print, 16 of them read by the evaluator.
    // The final independent PR #394 review changes that imported snapshot
    // by a net 83 non-mana activations, 36 of them read. Recount the whole
    // retained pool after canonical holds and complete optional removals.
    assert_eq!((read, of), (2783, 3490), "activated/loyalty reader census");

    // And one sentence it has no row for stays unread: Aminatou's −6
    // exchanges every nontoken permanent's controller around the table,
    // which is worth what the table looks like after, and nothing here
    // works that out.
    let aminatou = baylee_cards::decks::by_name("Aminatou, the Fateshifter").unwrap();
    let Some(AbilityDef::Loyalty { effects, .. }) = baylee_cards::by_index(aminatou)
        .unwrap()
        .abilities_for_face(0)
        .get(2)
    else {
        panic!("Aminatou's third ability is her −6");
    };
    assert_eq!(
        agent().effects_worth(&v, origin, effects, Aim::Source, 0),
        None
    );
}
