use super::*;

#[test]
fn witness_replacement_preserves_distinct_routes_and_existing_families() {
    let proof = |signature, family| Proof {
        record: Vec::new(),
        record_digest: [0; 32],
        strategy_digest: [signature; 32],
        family,
        turn: 9,
    };
    let proofs = vec![
        proof(1, Family::Combat),
        proof(2, Family::Combat),
        proof(3, Family::Combat),
    ];
    assert_eq!(proof_slot(&proofs, Family::DirectDamage, [1; 32]), Some(0));
    assert_eq!(proof_slot(&proofs, Family::Combat, [1; 32]), None);
    assert_eq!(proof_slot(&proofs, Family::Combat, [4; 32]), Some(3));
    let proofs = vec![proof(1, Family::DirectDamage), proof(2, Family::Combat)];
    assert_eq!(proof_slot(&proofs, Family::Combat, [1; 32]), None);
    assert_eq!(proof_slot(&proofs[..1], Family::Combat, [1; 32]), None);
}

#[test]
fn daily_has_three_fresh_replayable_resource_routes() {
    let result = generate(Kind::Daily, 41, Limits::default()).unwrap();
    assert_eq!(result.proofs.len(), 3);
    assert_eq!(
        result
            .proofs
            .iter()
            .map(|proof| proof.family)
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
    let bytes = result.package.encode().unwrap();
    assert!(Package::decode(&bytes, baylee_build::short()).is_ok());
    assert!(Package::decode(&bytes, "another engine").is_err());
}

#[test]
fn daily_rotates_meaningfully_different_resource_positions() {
    let generator = Generator::from_source(&crate::working::repo_root()).unwrap();
    let mut positions = BTreeSet::new();
    for seed in 41..45 {
        let result = generator
            .generate(Kind::Daily, seed, Limits::default())
            .unwrap();
        assert_eq!(result.proofs.len(), 3, "seed {seed}");
        assert!(result.proofs.iter().all(|proof| proof.turn == 1));
        let seats = &result.package.preset.seats;
        assert!(positions.insert((
            seats[0].starting_battlefield.len(),
            seats[1].starting_life,
            seats[1].starting_battlefield.len(),
        )));
    }
}

#[test]
fn unfinished_games_never_become_a_certificate() {
    assert!(
        generate(
            Kind::Daily,
            41,
            Limits {
                answers: 1,
                turns: 1
            }
        )
        .is_err()
    );
}

#[test]
fn weekly_uses_generated_decks_and_has_verified_combat_and_damage_finishes() {
    let result = generate(Kind::Weekly, 63, Limits::default()).unwrap();
    assert!(result.proofs.len() >= 3);
    assert!(result.proofs.iter().all(|proof| proof.turn >= 9));
    assert!(!result.deck.contains("Mana Crypt"));
    assert!(!result.deck.contains("Sol Ring"));
    assert!(!result.deck.contains("Black Lotus"));
    assert_eq!(
        result
            .proofs
            .iter()
            .map(|proof| proof.family)
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
    assert!(
        result
            .package
            .preset
            .seats
            .iter()
            .all(|seat| seat.starting_hand.is_none() && seat.starting_battlefield.is_empty())
    );
    let period = central_api::challenges::Period::containing(Kind::Weekly, 1_794_182_400).unwrap();
    let publication = result.publication(period).unwrap();
    assert!(Package::from_publication(&publication).is_ok());
    let mut corrupted = publication;
    corrupted.package[0] ^= 1;
    assert!(Package::from_publication(&corrupted).is_err());
}

#[test]
fn automatic_stock_recovers_an_ambiguous_publish_then_rolls_over_once() {
    use baylee_challenges::stock;
    use central_api::challenges::{Challenge, HostRequest, HostResponse};
    use std::cell::RefCell;

    let daily = generate(Kind::Daily, 41, Limits::default()).unwrap();
    let weekly = generate(Kind::Weekly, 63, Limits::default()).unwrap();
    let monday = 1_794_182_400;
    let tasks = RefCell::new(Vec::<Challenge>::new());
    let prepared = RefCell::new(Vec::new());
    let mut lose_reply = true;
    let mut exchange = |request| match request {
        HostRequest::Inventory { from, until } => Ok(HostResponse::Inventory {
            challenges: tasks
                .borrow()
                .iter()
                .filter(|task| {
                    let starts = task.description.period.starts_at;
                    starts >= from && starts < until
                })
                .cloned()
                .collect(),
        }),
        HostRequest::Publish { publication } => {
            publication.validate().unwrap();
            assert!(
                !tasks
                    .borrow()
                    .iter()
                    .any(|task| task.description.period == publication.description.period)
            );
            let challenge = Challenge {
                id: *uuid::Uuid::now_v7().as_bytes(),
                description: publication.description,
                digest: publication.digest,
                withdrawn: false,
            };
            tasks.borrow_mut().push(challenge.clone());
            if lose_reply {
                lose_reply = false;
                Err(central_api::Error::Unavailable)
            } else {
                Ok(HostResponse::Challenge { challenge })
            }
        }
        _ => panic!("stock never claims or scores a player attempt"),
    };
    let mut prepare = |period: central_api::challenges::Period| {
        prepared.borrow_mut().push(period);
        match period.kind {
            Kind::Daily => &daily,
            Kind::Weekly => &weekly,
        }
        .publication(period)
        .map_err(|_| stock::Error::Uncertified)
    };
    assert!(stock::reconcile(monday, 1, &mut exchange, &mut prepare).is_err());
    assert_eq!(
        tasks.borrow().len(),
        1,
        "the uncertain write really reached the authority"
    );
    assert_eq!(
        stock::reconcile(monday, 1, &mut exchange, &mut prepare).unwrap(),
        stock::Report {
            present: 1,
            published: 1
        }
    );
    assert_eq!(
        stock::reconcile(monday, 1, &mut exchange, &mut prepare).unwrap(),
        stock::Report {
            present: 2,
            published: 0
        }
    );
    assert_eq!(
        prepared.borrow().len(),
        2,
        "neither task was regenerated after the lost response"
    );
    assert_eq!(
        stock::reconcile(monday + 86_400, 1, &mut exchange, &mut prepare).unwrap(),
        stock::Report {
            present: 1,
            published: 1
        }
    );
    assert_eq!(tasks.borrow().len(), 3);
    assert_eq!(prepared.borrow().len(), 3);
}
