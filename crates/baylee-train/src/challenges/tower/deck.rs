//! Curated upgrade axes plus a working-card-constrained opponent deck builder.

use crate::working::Working;
use anyhow::{Context, Result, ensure};
use baylee_cards::{
    deckgen::{Archetype, DeckBuildRequest, Pool, build_synergistic},
    decks::{house_deck, preset_for},
    formats::{self, Shape},
};
use baylee_core::{
    color::{Color, ColorSet},
    preset::{AIProfile, GamePreset, HouseRules, SeatController},
};
use central_api::{
    challenges::{Period, Words},
    tower::{Theme, Upgrade},
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

const STARTER: [(&str, u8); 11] = [
    ("Mountain", 12),
    ("Forest", 12),
    ("Grizzly Bears", 4),
    ("Hill Giant", 4),
    ("Llanowar Elves", 4),
    ("Shock", 4),
    ("Volcanic Hammer", 4),
    ("Giant Growth", 4),
    ("Lightning Bolt", 4),
    ("Raging Goblin", 4),
    ("Elvish Archers", 4),
];

pub(super) fn words(en: &str, de: &str) -> Words {
    Words {
        en: en.into(),
        de: de.into(),
    }
}

pub(super) fn theme(period: Period) -> Theme {
    let upgrade = |remove: &str, add: &str, title: Words, summary: Words| Upgrade {
        title,
        summary,
        copies: 2,
        remove: remove.into(),
        add: add.into(),
    };
    let assault = words("Strength of the wild", "Kraft der Wildnis");
    let precision = words("A sharper spark", "Der gezielte Funke");
    let resolve = words("Roots that endure", "Wurzeln, die standhalten");
    Theme {
        period,
        title: words("The verdant ascent", "Aufstieg im grünen Turm"),
        floors: [
            words("Rootbound gate", "Das Wurzeltor"),
            words("Ember gallery", "Die Glutgalerie"),
            words("The thorn sentinel", "Der Dornenwächter"),
            words("Canopy crossing", "Über den Baumkronen"),
            words("The crown guardian", "Der Hüter der Krone"),
        ],
        upgrades: [
            std::array::from_fn(|level| {
                if level < 2 {
                    upgrade(
                        "Grizzly Bears",
                        "Kird Ape",
                        assault.clone(),
                        words(
                            "A cheaper threat that grows beside a Forest. Keep both colours in play.",
                            "Eine günstigere Bedrohung, die mit einem Wald wächst. Halte beide Farben im Spiel.",
                        ),
                    )
                } else {
                    upgrade(
                        "Hill Giant",
                        "Blastoderm",
                        assault.clone(),
                        words(
                            "A larger, protected attacker for the same mana. Fading makes timing matter.",
                            "Ein größerer, geschützter Angreifer für dieselben Manakosten. Plane seinen Einsatz vor dem Verblassen.",
                        ),
                    )
                }
            }),
            std::array::from_fn(|level| {
                let (remove, add) = match level {
                    0 => ("Shock", "Incinerate"),
                    1 => ("Shock", "Chain Lightning"),
                    2 => ("Volcanic Hammer", "Incinerate"),
                    _ => ("Volcanic Hammer", "Chain Lightning"),
                };
                upgrade(
                    remove,
                    add,
                    precision.clone(),
                    words(
                        "Improve your damage spells. Compare mana cost and timing before choosing.",
                        "Verbessere deine Schadenszauber. Vergleiche Manakosten und Einsatzzeitpunkt vor der Wahl.",
                    ),
                )
            }),
            std::array::from_fn(|level| {
                if level < 2 {
                    upgrade(
                        "Elvish Archers",
                        "Strangleroot Geist",
                        resolve.clone(),
                        words(
                            "Haste and undying reward persistent attacks, but require two green mana.",
                            "Eile und Unverwüstlich belohnen ausdauernde Angriffe, benötigen aber zwei grüne Mana.",
                        ),
                    )
                } else {
                    upgrade(
                        "Raging Goblin",
                        "Wild Mongrel",
                        resolve.clone(),
                        words(
                            "Turn spare cards into power and protect your creature by changing its colour.",
                            "Mache überzählige Karten zu Stärke und schütze deine Kreatur durch einen Farbwechsel.",
                        ),
                    )
                }
            }),
        ],
    }
}

pub(super) fn player(theme: &Theme, counts: [u8; 3], working: &Working) -> Result<String> {
    ensure!(
        counts.iter().map(|&n| u16::from(n)).sum::<u16>() <= 4,
        "upgrade route exceeds four choices"
    );
    let mut cards: BTreeMap<String, u8> = STARTER
        .into_iter()
        .map(|(name, count)| (name.into(), count))
        .collect();
    for (track, count) in counts.into_iter().enumerate() {
        for upgrade in theme.upgrades[track].iter().take(usize::from(count)) {
            let prior = cards
                .get_mut(&upgrade.remove)
                .context("upgrade source absent")?;
            *prior = prior
                .checked_sub(upgrade.copies)
                .context("upgrade source exhausted")?;
            *cards.entry(upgrade.add.clone()).or_default() += upgrade.copies;
        }
    }
    let mut text = String::from("[deck:Verdant expedition]\n");
    for (name, count) in cards.into_iter().filter(|(_, count)| *count > 0) {
        let card = baylee_cards::decks::by_name(&name).context("unknown upgrade card")?;
        ensure!(
            working.check(card).is_ok(),
            "upgrade card lacks working evidence: {name}"
        );
        writeln!(text, "{count} {name}")?;
    }
    Ok(text)
}

pub(super) fn opponent_pool(working: &Working) -> Result<Pool> {
    // A bounded readable encounter vocabulary. The ordinary builder selects the actual
    // curve, mana base and mixture for each floor; no unsupported card is substituted.
    let names = [
        "Mountain",
        "Forest",
        "Grizzly Bears",
        "Raging Goblin",
        "Hill Giant",
        "Llanowar Elves",
        "Elvish Archers",
        "Craw Wurm",
        "Giant Spider",
        "Wall of Wood",
        "War Mammoth",
        "Shock",
        "Giant Growth",
        "Volcanic Hammer",
        "Rampant Growth",
    ];
    let mut cards = Vec::with_capacity(names.len());
    for name in names {
        let card = baylee_cards::decks::by_name(name)
            .with_context(|| format!("opponent card absent: {name}"))?;
        ensure!(
            working.check(card).is_ok(),
            "opponent card lacks working evidence: {name}"
        );
        cards.push(card);
    }
    Ok(Pool::from_cards(cards, &BTreeSet::new()))
}

pub(super) fn build(
    theme: &Theme,
    counts: [u8; 3],
    seed: u64,
    pool: &Pool,
    working: &Working,
) -> Result<(GamePreset, String)> {
    let floor: u8 = counts.iter().sum::<u8>() + 1;
    let text = player(theme, counts, working)?;
    let player = house_deck(&text).map_err(anyhow::Error::msg)?;
    let mut request = DeckBuildRequest::new(
        Shape::Constructed,
        Archetype::Midrange,
        baylee_challenges::seed(b"tower opponent deck", seed),
    );
    request.forced_colors = Some(ColorSet::from_slice(&[Color::Red, Color::Green]));
    let opponent = build_synergistic(pool, &request)?;
    let mut preset = preset_for(seed, &player, &opponent.loaded());
    // A campaign floor begins with playable colours and a real combat/burn decision.
    // The supplied hand is part of the immutable package, never a reroll or extra card.
    // Taking an assault upgrade also upgrades the opening threat itself.
    let opening = [
        "Mountain",
        if counts[2] == 0 { "Mountain" } else { "Forest" },
        "Forest",
        "Llanowar Elves",
        if counts[0] == 0 {
            "Grizzly Bears"
        } else {
            "Kird Ape"
        },
        "Lightning Bolt",
        if counts[1] == 0 {
            "Shock"
        } else {
            "Incinerate"
        },
    ];
    let hand = opening
        .into_iter()
        .map(|name| {
            let card = baylee_cards::decks::by_name(name).context("opening card absent")?;
            preset.seats[0]
                .deck
                .iter()
                .find(|entry| entry.card == card)
                .copied()
                .context("opening card not in supplied deck")
        })
        .collect::<Result<Vec<_>>>()?;
    for seat in &preset.seats {
        let main = seat.deck.iter().map(|entry| entry.card).collect::<Vec<_>>();
        ensure!(
            formats::check(Shape::Constructed, &main, &[]).is_empty(),
            "illegal tower deck"
        );
        ensure!(
            seat.deck
                .iter()
                .all(|entry| working.check(entry.card).is_ok()),
            "uncertified tower card"
        );
    }
    // Explicit starting zones are created separately by the engine. Remove the
    // seven physical copies from the library; otherwise this would be a 67-card game.
    for card in &hand {
        let index = preset.seats[0]
            .deck
            .iter()
            .position(|entry| entry == card)
            .context("opening copy exhausted")?;
        preset.seats[0].deck.remove(index);
    }
    preset.seats[0].starting_hand = Some(hand);
    preset.seats[0].controller = SeatController::Open;
    let profile = if floor < 3 {
        AIProfile::CASUAL
    } else {
        AIProfile::STEADY
    };
    preset.seats[1].controller = SeatController::Ai(AIProfile {
        foresight: 0,
        ..profile
    });
    preset.seats[1].starting_life = Some(12 + i32::from(floor - 1) * 3);
    preset.house_rules = HouseRules {
        challenge_policy_seed: Some(baylee_challenges::seed(b"tower opponent policy", seed)),
        starting_player: Some(0),
        decision_timeout_secs: 0,
        reconnect_window_secs: 0,
        time_extension_votes: false,
        takebacks: false,
        ..HouseRules::for_a_table()
    };
    Ok((preset, text))
}
