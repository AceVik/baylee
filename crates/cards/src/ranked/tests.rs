use super::*;
use baylee_cardtext::{CardLine, PoolFactsBody};

fn rows(rows: &[&str]) -> Vec<String> {
    rows.iter().map(|s| (*s).to_owned()).collect()
}

fn register(format: &str, cards: &[&str], side: &[&str], leaders: &[&str]) -> Registration {
    registration(format, &rows(cards), &rows(side), &rows(leaders)).unwrap()
}

fn facts(format: &str, verdicts: &[(&str, &str)]) -> PoolFacts {
    let lines = verdicts
        .iter()
        .map(|(name, status)| CardLine {
            index: crate::decks::by_name(name).unwrap().get(),
            legalities: [(format.to_owned(), (*status).to_owned())].into(),
            printed: Vec::new(),
        })
        .collect();
    PoolFactsBody::assemble(Vec::new(), lines).into()
}

#[test]
fn registration_binds_physical_choices_and_zones_but_elo_ignores_sideboarding() {
    let a = register("standard", &["59 Island", "1 Forest"], &["1 Swamp"], &[]);
    let b = register(
        "standard",
        &["1 Forest", "37 Island # ignored", "22 Island"],
        &["1 Swamp"],
        &[],
    );
    let swapped = register("standard", &["59 Island", "1 Swamp"], &["1 Forest"], &[]);
    assert_eq!(a.digest, b.digest);
    assert_eq!(a.deck.main, b.deck.main);
    let pool = rating_pool(&a);
    // The v1 rating identity for an ordinary (non-commander)
    // Forest stays readable after registry, worker or client upgrades.
    let forest = [
        0x01, 0xba, 0xc6, 0xa8, 0xc3, 0x54, 0x54, 0x8a, 0x12, 0x6a, 0x9e, 0xe0, 0xdc, 0x73, 0xf0,
        0x3c, 0xbf, 0xa8, 0xa9, 0x5a, 0x3a, 0xb8, 0x14, 0x98, 0xc1, 0x0c, 0x13, 0x16, 0x9c, 0xc2,
        0x78, 0x8b,
    ];
    assert!(pool.contains(&(forest, 1)));
    assert!(pool.windows(2).all(|pair| pair[0].0 < pair[1].0));
    assert_eq!(pool.iter().map(|entry| entry.1).sum::<u16>(), 61);
    assert_eq!(pool, rating_pool(&swapped));
    assert_ne!(a.digest, swapped.digest);
    let alt = register(
        "standard",
        &[
            "59 Island",
            "1 Forest scryfall=00000000-0000-0000-0000-000000000001",
        ],
        &["1 Swamp"],
        &[],
    );
    assert_eq!(pool, rating_pool(&alt));
    assert_ne!(a.digest, alt.digest);
    let wu = register("highlander", &["1 Cryptic Spires circles=WU"], &[], &[]);
    let wr = register("highlander", &["1 Cryptic Spires circles=WR"], &[], &[]);
    assert_ne!(wu.digest, wr.digest);
    assert_eq!(rating_pool(&wu), rating_pool(&wr));
}

#[test]
fn format_catalog_is_authoritative_for_every_zone() {
    let deck = register("standard", &["60 Forest"], &["1 Island"], &[]).deck;
    let good = facts("standard", &[("Forest", "legal"), ("Island", "legal")]);
    assert!(admit("standard", &deck, &good).is_ok());
    for catalog in [
        facts("modern", &[("Forest", "legal"), ("Island", "legal")]),
        facts("standard", &[("Forest", "legal")]),
        facts("standard", &[("Forest", "legal"), ("Island", "not_legal")]),
        facts("standard", &[("Forest", "legal"), ("Island", "banned")]),
    ] {
        assert!(admit("standard", &deck, &catalog).is_err());
    }
    let short = register("standard", &["59 Forest"], &[], &[]);
    assert!(admit("standard", &short.deck, &good).is_err());
}

#[test]
fn ranked_host_rows_hide_personal_notes_and_retain_the_playable_commitment() {
    let registered = register(
        "highlander",
        &[
            "1 Forest (UNH) 140 [de] *F* # Alice's private list",
            "1 Cryptic Spires circles=WU",
        ],
        &["1 Swamp # secret"],
        &["Forest"],
    );
    let [main, side, commanders] = playable_rows(&registered.deck).unwrap();
    assert!(
        main.iter()
            .chain(&side)
            .all(|line| !line.contains('#') && !line.contains("UNH") && !line.contains("[de]"))
    );
    let restored = registration("highlander", &main, &side, &commanders).unwrap();
    assert_eq!(restored.digest, registered.digest);
    assert_eq!(rating_pool(&restored), rating_pool(&registered));
    assert_eq!(commanders, ["Forest"]);
}

#[test]
fn highlander_has_its_own_shape_even_when_a_vintage_verdict_is_legal() {
    let catalog = facts("vintage", &[("Forest", "legal"), ("Sol Ring", "legal")]);
    let good = register("highlander", &["99 Forest", "1 Sol Ring"], &[], &[]);
    assert!(admit("highlander", &good.deck, &catalog).is_ok());
    for bad in [
        register("highlander", &["99 Forest"], &[], &[]),
        register(
            "highlander",
            &["99 Forest", "1 Sol Ring"],
            &["1 Sol Ring"],
            &[],
        ),
        register("highlander", &["100 Forest"], &[], &["Sol Ring"]),
    ] {
        assert!(admit("highlander", &bad.deck, &catalog).is_err());
    }
}

#[test]
fn designated_commanders_have_a_distinct_role_and_are_taken_once() {
    let name = crate::all()
        .find(|def| {
            matches!(
                def.coverage,
                crate::dsl::Coverage::Generated | crate::dsl::Coverage::Implemented
            ) && crate::decks::leader_of(def.index).is_some_and(|l| l.eligible_alone)
                && def
                    .color_identity
                    .contains(baylee_core::color::Color::Green)
        })
        .unwrap()
        .faces[0]
        .name;
    let marked = format!("1 {name}");
    let a = register("commander", &["99 Forest", &marked], &[], &[name]);
    let b = register("commander", &["99 Forest"], &[], &[name]);
    let plain = register("commander", &["99 Forest", &marked], &[], &[]);
    assert_eq!(a.digest, b.digest);
    assert_eq!(a.deck.main.len(), 99);
    assert_ne!(rating_pool(&a), rating_pool(&plain));
    let catalog = facts("commander", &[("Forest", "legal"), (name, "legal")]);
    assert!(admit("commander", &a.deck, &catalog).is_ok());
    assert!(admit("commander", &plain.deck, &catalog).is_err());
    assert!(
        registration(
            "commander",
            &rows(&["99 Forest"]),
            &[],
            &rows(&[name, name])
        )
        .is_err()
    );
}

#[test]
fn malformed_rows_and_huge_counts_are_rejected_before_expansion() {
    for row in [
        "4294967295 Forest",
        "0 Forest",
        "1 Not A Card",
        "1 Forest scryfall=no",
        "1 Forest circles=WU",
    ] {
        assert!(
            registration("standard", &rows(&[row]), &[], &[]).is_err(),
            "{row}"
        );
    }
    assert!(
        registration(
            "standard",
            &rows(&["60 Forest"]),
            &rows(&["16 Island"]),
            &[]
        )
        .is_err()
    );
    assert!(registration("unknown", &rows(&["60 Forest"]), &[], &[]).is_err());
}
