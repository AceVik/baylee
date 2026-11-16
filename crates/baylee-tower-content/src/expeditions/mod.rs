//! Curated build-arounds, assembled once by the shared deterministic deck builder.
//! Publication freezes the resulting cards: a registry update never edits a run.
use super::{Card, Catalog, Collection, Content, Error, Formation, Stack, metadata, named};
use baylee_cards::deckgen::{Archetype, Pool, build_commander_run_with_core};
use baylee_core::ids::CardIndex;
use std::collections::{BTreeMap, BTreeSet};

mod lands;
mod theme;

// Lower commander costs and several distinct plans per colour pair avoid a pool
// made solely of expensive finishers. Seeds are explicit and append-only.
const LEADERS: [(&str, Archetype); 50] = [
    ("Emmara, Soul of the Accord", Archetype::Aggro),
    ("Elas il-Kor, Sadistic Pilgrim", Archetype::Midrange),
    ("Talrand, Sky Summoner", Archetype::Control),
    ("Slimefoot, the Stowaway", Archetype::Midrange),
    ("Pia and Kiran Nalaar", Archetype::Midrange),
    ("Radha, Heir to Keld", Archetype::Aggro),
    ("Rishkar, Peema Renegade", Archetype::Ramp),
    ("Sram, Senior Edificer", Archetype::Aggro),
    ("Hapatra, Vizier of Poisons", Archetype::Midrange),
    ("Tatyova, Benthic Druid", Archetype::Ramp),
    ("Judith, the Scourge Diva", Archetype::Aggro),
    ("Kykar, Wind's Fury", Archetype::Control),
    ("Alela, Artful Provocateur", Archetype::Midrange),
    ("Rin and Seri, Inseparable", Archetype::Midrange),
    ("Giada, Font of Hope", Archetype::Ramp),
    ("Glissa, the Traitor", Archetype::Midrange),
    ("Thrun, the Last Troll", Archetype::Aggro),
    ("Derevi, Empyrial Tactician", Archetype::Aggro),
    ("Titania, Protector of Argoth", Archetype::Ramp),
    ("Dwynen, Gilt-Leaf Daen", Archetype::Aggro),
    ("Kambal, Consul of Allocation", Archetype::Control),
    ("Padeem, Consul of Innovation", Archetype::Control),
    ("Yahenni, Undying Partisan", Archetype::Aggro),
    ("Samut, Voice of Dissent", Archetype::Aggro),
    ("Kess, Dissident Mage", Archetype::Control),
    ("Vona, Butcher of Magan", Archetype::Midrange),
    ("Arvad the Cursed", Archetype::Midrange),
    ("Garna, the Bloodflame", Archetype::Midrange),
    ("Lyra Dawnbringer", Archetype::Midrange),
    ("Muldrotha, the Gravetide", Archetype::Ramp),
    ("Pashalik Mons", Archetype::Aggro),
    ("Ayara, First of Locthwain", Archetype::Midrange),
    ("Emry, Lurker of the Loch", Archetype::Midrange),
    ("Linden, the Steadfast Queen", Archetype::Aggro),
    ("Arasta of the Endless Web", Archetype::Midrange),
    ("Eutropia the Twice-Favored", Archetype::Midrange),
    ("Chevill, Bane of Monsters", Archetype::Control),
    ("Kaervek, the Spiteful", Archetype::Control),
    ("Emiel the Blessed", Archetype::Midrange),
    ("Ashaya, Soul of the Wild", Archetype::Ramp),
    ("Ghen, Arcanum Weaver", Archetype::Midrange),
    ("Birgi, God of Storytelling", Archetype::Aggro),
    ("Maja, Bretagard Protector", Archetype::Midrange),
    ("Galazeth Prismari", Archetype::Control),
    ("Grist, the Hunger Tide", Archetype::Midrange),
    ("Sythis, Harvest's Hand", Archetype::Midrange),
    ("Saryth, the Viper's Fang", Archetype::Midrange),
    ("Mahadi, Emporium Master", Archetype::Midrange),
    ("Ertai Resurrected", Archetype::Control),
    ("Tura Kennerüd, Skyknight", Archetype::Control),
];

// Keep unusually explosive openings out of a short starter deck. These are
// construction exclusions, not altered Magic rules or a competitive banlist.
const EXCLUDED: &[&str] = &[
    "Cryptic Spires",
    "Black Lotus",
    "Mox Pearl",
    "Mox Sapphire",
    "Mox Jet",
    "Mox Ruby",
    "Mox Emerald",
    "Mox Diamond",
    "Chrome Mox",
    "Mana Crypt",
    "Mana Vault",
    "Sol Ring",
    "Ancient Tomb",
    "City of Traitors",
    "Jeweled Lotus",
    "Lion's Eye Diamond",
    "Lotus Petal",
    "Time Walk",
    "Ancestral Recall",
    "Timetwister",
    "Channel",
    "Tinker",
    "Demonic Consultation",
    "Thassa's Oracle",
];

/// Prepare fifty 40-card build-arounds with singleton spells, reviewed utility
/// lands and a colour-correct mana base. This is an offline publication operation;
/// it never runs per socket, per frame or during an engine turn.
/// # Errors
/// Missing native support, invalid generated deck or publication capacity.
pub fn expeditions() -> Result<Content, Error> {
    let excluded: BTreeSet<_> = EXCLUDED
        .iter()
        .filter_map(|name| baylee_cards::decks::by_name(name))
        .collect();
    let mut pool = Pool::all_implemented(&excluded);
    lands::prepare(&mut pool);
    let mut cards = BTreeMap::<CardIndex, Card>::new();
    for name in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        let card = named(name)?;
        cards.insert(card.card, card);
    }
    let mut starters = Vec::with_capacity(LEADERS.len());
    for (index, &(name, archetype)) in LEADERS.iter().enumerate() {
        let commander = named(name)?.card;
        let seed = 0x746f_7765_7200_0000 | u64::try_from(index).map_err(|_| Error::Formation)?;
        let core = theme::core(&pool, commander, name, seed)?;
        let mut generated =
            build_commander_run_with_core(&pool, commander, archetype, seed, &core)?;
        lands::retain_fetch_targets(
            &mut generated.main,
            baylee_cards::by_index(commander)
                .ok_or(Error::Metadata)?
                .color_identity,
        )?;
        let mut counts = BTreeMap::<CardIndex, u8>::new();
        let mut owned = Vec::new();
        for index in std::iter::once(commander).chain(generated.main.iter().copied()) {
            let card = metadata(index).ok_or(Error::Metadata)?;
            cards.insert(index, card);
            if !card.basic_land {
                owned.push(Stack {
                    card: index,
                    copies: 1,
                });
            }
        }
        for index in generated.main {
            *counts.entry(index).or_default() += 1;
        }
        owned.sort_by_key(|row| row.card);
        starters.push(Collection {
            formation: Some(Formation {
                commander,
                sideboard: Vec::new(),
            }),
            owned,
            deck: counts
                .into_iter()
                .map(|(card, copies)| Stack { card, copies })
                .collect(),
        });
    }
    let catalog = Catalog {
        cards: cards.into_values().collect(),
    };
    catalog
        .validate_starter_pool(&starters)
        .map_err(|_| Error::Formation)?;
    Ok(Content { catalog, starters })
}
