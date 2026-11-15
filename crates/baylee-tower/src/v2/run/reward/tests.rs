use super::*;
use crate::{
    cards::Stack,
    v2::{
        generation,
        package::{Build, Cohort, Opponent, Origin, Seeds},
        run::{Binding, Command, WorkerCommand},
    },
};
use baylee_protocol::execution::{Identity, Variant};

const SEED: [u8; 32] = [43; 32];

fn fixture() -> (Catalog, Run, Cohort, Build) {
    let (mut catalog, cards) = crate::cards::run_tests::fixture();
    for id in 10..18 {
        catalog.cards.push(crate::cards::Card {
            card: CardIndex::new(id),
            ..catalog.cards[6]
        });
    }
    let run = Run::begin_for(&catalog, cards, &SEED, generation::RUN).unwrap();
    let mut identity = Identity::native(Variant::Tower, "reward-test", "synthetic", 21);
    identity.rules = "tower-5".into();
    let cohort = Cohort::for_generation(identity, &catalog, generation::RUN).unwrap();
    let mut enemy = Build::freeze(&run, &catalog).unwrap();
    enemy.deck[0].copies -= 8;
    enemy.deck.extend((10..18).map(|card| Stack {
        card: CardIndex::new(card),
        copies: 1,
    }));
    (catalog, run, cohort, enemy)
}

fn verified(
    run: &Run,
    catalog: &Catalog,
    cohort: &Cohort,
    build: &Build,
    outcome: Outcome,
) -> (Run, Package, Settlement) {
    let reserved = Package::reserve(
        run,
        run.revision,
        catalog,
        cohort,
        Opponent {
            stage: run.shop.stage,
            cohort: cohort.clone(),
            origin: Origin::House { archetype: 5 },
            build: build.clone(),
        },
        Seeds {
            shuffle: u64::from(run.shop.stage),
            policy: 9,
        },
        4096,
    )
    .unwrap();
    let run = reserved.run;
    let run = run
        .worker(
            run.revision,
            10,
            WorkerCommand::Claim { duration: 1000 },
            catalog,
        )
        .unwrap();
    let execution = run.execution.as_ref().unwrap();
    let run = run
        .worker(
            run.revision,
            11,
            WorkerCommand::Append {
                fence: execution.fence,
                expected: execution.prefix,
                chunk: b"synthetic record",
            },
            catalog,
        )
        .unwrap();
    let execution = run.execution.as_ref().unwrap();
    let run = run
        .worker(
            run.revision,
            12,
            WorkerCommand::Seal {
                fence: execution.fence,
                expected: execution.prefix,
            },
            catalog,
        )
        .unwrap();
    let execution = run.execution.as_ref().unwrap();
    let result = Settlement {
        binding: Binding {
            package: execution.package,
            fence: execution.fence,
            prefix: execution.prefix,
        },
        outcome,
        turns: 31,
        used: [0; 2],
    };
    (run, reserved.package, result)
}

#[test]
fn reward_uses_exact_opponent_deck_and_stays_saved_until_valid_choice() {
    let (catalog, run, cohort, enemy) = fixture();
    let frozen = run
        .command(0, &Command::Freeze { frozen: true }, &catalog, &SEED)
        .unwrap();
    let (playing, package, result) = verified(&frozen, &catalog, &cohort, &enemy, Outcome::Victory);
    assert!(
        playing
            .settled(playing.revision, result.clone(), &catalog, &SEED)
            .is_err()
    );
    let pending = playing
        .settled_against(playing.revision, result.clone(), &catalog, &SEED, &package)
        .unwrap();
    assert_eq!(pending.phase, Phase::Reward);
    assert_eq!(
        pending.shop, frozen.shop,
        "shop does not roll before the choice"
    );
    assert_eq!(pending.economy.gold, run.economy.gold + 12);
    let reward = pending.reward.as_ref().unwrap();
    assert_eq!(reward.cards.len(), 5);
    assert_eq!(reward.take, 3);
    assert!(
        reward
            .cards
            .iter()
            .all(|card| *card != CardIndex::new(9) && *card != CardIndex::new(4))
    );
    assert!(reward.cards.iter().all(
        |card| enemy.deck.iter().any(|stack| stack.card == *card) || *card == CardIndex::new(5)
    ));
    let encoded = serde_json::to_vec(&pending).unwrap();
    let restored: Run = serde_json::from_slice(&encoded).unwrap();
    restored.validate(&catalog).unwrap();
    assert_eq!(restored, pending);
    assert!(
        pending
            .settled_against(pending.revision, result, &catalog, &SEED, &package)
            .is_err()
    );
    for cards in [
        vec![],
        reward.cards[..2].to_vec(),
        vec![reward.cards[0]; 3],
        vec![CardIndex::new(999); 3],
    ] {
        assert!(
            pending
                .command(pending.revision, &Command::Claim { cards }, &catalog, &SEED)
                .is_err()
        );
    }
    assert!(
        pending
            .command(pending.revision, &Command::Refresh {}, &catalog, &SEED)
            .is_err()
    );
    assert!(
        pending
            .command(pending.revision, &Command::Abandon {}, &catalog, &SEED)
            .is_err()
    );
    let command = Command::Claim {
        cards: reward.cards[..3].to_vec(),
    };
    let claimed = pending
        .command(pending.revision, &command, &catalog, &SEED)
        .unwrap();
    assert_eq!(claimed.phase, Phase::Shop);
    assert_eq!(claimed.shop.stage, 2);
    assert!(claimed.shop.frozen);
    assert_eq!(claimed.shop.offers, pending.shop.offers);
    assert_eq!(claimed.claims.len(), 1);
    assert!(claimed.reward.is_none());
    assert!(
        claimed
            .command(claimed.revision, &command, &catalog, &SEED)
            .is_err()
    );
    assert!(
        claimed
            .command(pending.revision, &command, &catalog, &SEED)
            .is_err()
    );
}

#[test]
fn duplicate_cards_yield_fixed_gold_and_fewer_candidates_reduce_choice() {
    let (catalog, run, cohort, _) = fixture();
    let enemy = Build::freeze(&run, &catalog).unwrap();
    let (run, package, result) = verified(&run, &catalog, &cohort, &enemy, Outcome::Victory);
    let run = run
        .settled_against(run.revision, result, &catalog, &SEED, &package)
        .unwrap();
    let reward = run.reward.as_ref().unwrap();
    assert_eq!(reward.cards.len(), 3);
    assert_eq!(reward.take, 3);
    let claimed = run
        .command(
            run.revision,
            &Command::Claim {
                cards: reward.cards.clone(),
            },
            &catalog,
            &SEED,
        )
        .unwrap();
    assert_eq!(claimed.cards, run.cards);
    assert_eq!(claimed.economy.gold, run.economy.gold + 3);
    assert_eq!(claimed.claims[0].duplicates, 3);
    claimed.validate(&catalog).unwrap();
    let mut forged = claimed;
    forged.claims[0].duplicates = 4;
    assert!(forged.validate(&catalog).is_err());
}

#[test]
fn terminal_victory_and_defeat_require_choice_without_a_phantom_shop() {
    for (outcome, rounds, phase) in [
        (Outcome::Victory, 8, Phase::Conquered),
        (Outcome::Defeat, 5, Phase::Exhausted),
    ] {
        let (catalog, mut run, cohort, enemy) = fixture();
        for stage in 1..=rounds {
            let (sealed, package, result) = verified(&run, &catalog, &cohort, &enemy, outcome);
            let pending = sealed
                .settled_against(sealed.revision, result, &catalog, &SEED, &package)
                .unwrap();
            let reward = pending.reward.as_ref().unwrap();
            assert_eq!(reward.take, if outcome == Outcome::Victory { 3 } else { 1 });
            let command = Command::Claim {
                cards: reward.cards[..usize::from(reward.take)].to_vec(),
            };
            run = pending
                .command(pending.revision, &command, &catalog, &SEED)
                .unwrap();
            if stage == rounds {
                assert_eq!(run.phase, phase);
                assert_eq!(run.shop.stage, rounds);
                assert_eq!(run.shop.offers, pending.shop.offers);
            } else {
                assert_eq!(run.phase, Phase::Shop);
                assert_eq!(run.shop.stage, stage + 1);
                assert!(
                    run.shop
                        .offers
                        .iter()
                        .zip(&pending.shop.offers)
                        .all(|(next, prior)| next.id != prior.id)
                );
            }
        }
    }
}

#[test]
fn another_package_or_changed_deck_cannot_supply_rewards() {
    let (catalog, run, cohort, enemy) = fixture();
    let (run, mut package, result) = verified(&run, &catalog, &cohort, &enemy, Outcome::Defeat);
    package.shuffle_seed += 1;
    assert!(
        run.settled_against(run.revision, result.clone(), &catalog, &SEED, &package)
            .is_err()
    );
    package.shuffle_seed -= 1;
    package.builds[1].deck[0].copies -= 1;
    assert!(
        run.settled_against(run.revision, result, &catalog, &SEED, &package)
            .is_err()
    );
}
