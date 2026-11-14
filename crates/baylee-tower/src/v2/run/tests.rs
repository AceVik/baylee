use super::*;
use crate::{
    cards::{Card, Stack},
    v2::{
        catalog::{Definition, Family, Rarity},
        economy::Recipe,
        inventory::Slot,
        shop::{Delivery, Goods},
    },
};
use baylee_core::ids::CardIndex;

const SEED: [u8; 32] = [19; 32];

fn fixture() -> (Catalog, Run) {
    let mut cards = vec![Card {
        card: CardIndex::new(0),
        price: 0,
        basic_land: true,
    }];
    cards.extend((1..=8).map(|id| Card {
        card: CardIndex::new(id),
        price: 2,
        basic_land: false,
    }));
    let catalog = Catalog { cards };
    let starter = Collection {
        owned: Vec::new(),
        deck: vec![Stack {
            card: CardIndex::new(0),
            copies: 40,
        }],
    };
    let run = Run::begin(&catalog, starter, &SEED).unwrap();
    (catalog, run)
}

fn playing(run: &Run, catalog: &Catalog) -> Run {
    let package = [run.shop.stage; 32];
    let run = run.reserved(run.revision, package, 4096, catalog).unwrap();
    run.worker(
        run.revision,
        100,
        WorkerCommand::Claim { duration: 1000 },
        catalog,
    )
    .unwrap()
}

fn recorded(run: &Run, catalog: &Catalog) -> Run {
    let execution = run.execution.as_ref().unwrap();
    run.worker(
        run.revision,
        101,
        WorkerCommand::Append {
            fence: execution.fence,
            expected: execution.prefix,
            chunk: b"synthetic domain record\n",
        },
        catalog,
    )
    .unwrap()
}

fn sealed(run: &Run, catalog: &Catalog) -> Run {
    let execution = run.execution.as_ref().unwrap();
    run.worker(
        run.revision,
        102,
        WorkerCommand::Seal {
            fence: execution.fence,
            expected: execution.prefix,
        },
        catalog,
    )
    .unwrap()
}

fn result(run: &Run, outcome: Outcome) -> Settlement {
    let execution = run.execution.as_ref().unwrap();
    Settlement {
        binding: Binding {
            package: execution.package,
            fence: execution.fence,
            prefix: execution.prefix,
        },
        outcome,
        turns: 1000,
        used: [0; 2],
    }
}

fn advance(run: &Run, outcome: Outcome, catalog: &Catalog) -> Run {
    let run = sealed(&recorded(&playing(run, catalog), catalog), catalog);
    run.settled(run.revision, result(&run, outcome), catalog, &SEED)
        .unwrap()
}

#[test]
fn only_a_sealed_exact_current_result_grants_income_once() {
    let (catalog, initial) = fixture();
    let playing = playing(&initial, &catalog);
    for command in [
        Command::Refresh,
        Command::Abandon,
        Command::Edit {
            deck: initial.cards.deck.clone(),
        },
    ] {
        assert!(
            playing
                .command(playing.revision, &command, &catalog, &SEED)
                .is_err()
        );
    }
    let recorded = recorded(&playing, &catalog);
    assert!(
        recorded
            .settled(
                recorded.revision,
                result(&recorded, Outcome::Victory),
                &catalog,
                &SEED
            )
            .is_err()
    );
    let sealed = sealed(&recorded, &catalog);
    let original = result(&sealed, Outcome::Victory);
    for variant in 0..4 {
        let mut bad = original.clone();
        match variant {
            0 => bad.binding.package[0] ^= 1,
            1 => bad.binding.fence += 1,
            2 => bad.binding.prefix.digest[0] ^= 1,
            _ => bad.used = [1, 0],
        }
        assert!(
            sealed
                .settled(sealed.revision, bad, &catalog, &SEED)
                .is_err()
        );
    }
    let finished = sealed
        .settled(sealed.revision, original.clone(), &catalog, &SEED)
        .unwrap();
    assert_eq!(finished.phase, Phase::Shop);
    assert_eq!(finished.economy.gold, STARTING_GOLD + DUEL_INCOME);
    assert_eq!(finished.history.len(), 1);
    assert_eq!(finished.shop.stage, 2);
    assert!(
        finished
            .settled(finished.revision, original, &catalog, &SEED)
            .is_err()
    );
    assert!(
        finished
            .reserved(finished.revision, [1; 32], 4096, &catalog)
            .is_err()
    );
    let restored: Run = serde_json::from_slice(&serde_json::to_vec(&finished).unwrap()).unwrap();
    assert_eq!(restored, finished);
    restored.validate(&catalog).unwrap();
}

#[test]
fn twelfth_duel_ends_both_boundary_paths_and_long_games_are_not_rules_losses() {
    for last in [Outcome::Victory, Outcome::Draw] {
        let (catalog, mut run) = fixture();
        for outcome in [Outcome::Victory; 7]
            .into_iter()
            .chain([Outcome::Defeat; 4])
            .chain([last])
        {
            run = advance(&run, outcome, &catalog);
        }
        assert_eq!(run.history.len(), 12);
        assert_eq!(run.shop.stage, 12);
        assert_eq!(run.economy.gold, 162);
        assert_eq!(
            run.history.iter().map(|result| result.turns).sum::<u32>(),
            12_000
        );
        assert_eq!(
            run.phase,
            if last == Outcome::Victory {
                Phase::Conquered
            } else {
                Phase::Exhausted
            }
        );
        assert!(
            run.reserved(run.revision, [13; 32], 4096, &catalog)
                .is_err()
        );
        assert!(
            run.command(run.revision, &Command::Refresh, &catalog, &SEED)
                .is_err()
        );
    }
    let (catalog, mut run) = fixture();
    for _ in 0..5 {
        let sealed = sealed(&recorded(&playing(&run, &catalog), &catalog), &catalog);
        let mut concession = result(&sealed, Outcome::Defeat);
        concession.turns = 0;
        run = sealed
            .settled(sealed.revision, concession, &catalog, &SEED)
            .unwrap();
    }
    assert_eq!(run.phase, Phase::Exhausted);
    assert_eq!(run.economy.gold, 78);
}

#[test]
fn actual_shop_commands_conserve_paid_basis_and_preserve_pins_between_duels() {
    let (catalog, mut run) = fixture();
    // An explicit valid saved offer fixture isolates command/ledger behavior;
    // generation distribution is tested separately, not asserted by this case.
    for offer in &mut run.shop.offers[3..6] {
        offer.goods = Goods::Relic(Definition {
            family: Family::Grove,
            rarity: Rarity::Common,
        });
    }
    let pin = run.shop.offers[0].clone();
    run = run
        .command(
            run.revision,
            &Command::Pin {
                offer: pin.id,
                pinned: true,
            },
            &catalog,
            &SEED,
        )
        .unwrap();
    for slot in 0..3 {
        let offer = run.shop.offers[3 + usize::from(slot)].id;
        let command = Command::Buy {
            offer,
            delivery: Delivery::Slot(Slot::Reserve(slot)),
        };
        let old = run.revision;
        run = run.command(old, &command, &catalog, &SEED).unwrap();
        assert!(run.command(old, &command, &catalog, &SEED).is_err());
        assert!(
            run.command(run.revision, &command, &catalog, &SEED)
                .is_err()
        );
    }
    let ingredients = run
        .economy
        .inventory
        .relics
        .iter()
        .map(|item| item.id)
        .collect();
    run = run
        .command(
            run.revision,
            &Command::Fuse {
                recipe: Recipe {
                    relics: ingredients,
                    mythic: None,
                    destination: Slot::Reserve(0),
                },
            },
            &catalog,
            &SEED,
        )
        .unwrap();
    let item = run.economy.inventory.relics[0].id;
    assert_eq!(run.economy.inventory.relics[0].basis, 12);
    run = run
        .command(
            run.revision,
            &Command::Move {
                item,
                slot: Slot::Active(0),
            },
            &catalog,
            &SEED,
        )
        .unwrap();
    run = run
        .command(run.revision, &Command::Sell { item }, &catalog, &SEED)
        .unwrap();
    assert_eq!(run.economy.gold, 11);
    assert_eq!(
        run.ledger,
        Ledger {
            spent: 12,
            resold: 5
        }
    );
    assert!(
        run.command(run.revision, &Command::Sell { item }, &catalog, &SEED)
            .is_err()
    );
    run = advance(&run, Outcome::Defeat, &catalog);
    assert_eq!(run.shop.offers[0].id, pin.id);
    assert!(run.shop.offers[0].pinned);
    assert_eq!(run.economy.gold, 23);
}

#[test]
fn quarantine_preserves_the_package_and_prefix_and_fences_old_verification() {
    let (catalog, initial) = fixture();
    let live = recorded(&playing(&initial, &catalog), &catalog);
    let old = live.execution.as_ref().unwrap();
    let stopped = live.quarantined(live.revision, &catalog).unwrap();
    assert!(
        stopped
            .reserved(stopped.revision, [9; 32], 4096, &catalog)
            .is_err()
    );
    assert!(
        stopped
            .command(stopped.revision, &Command::Abandon, &catalog, &SEED)
            .is_err()
    );
    let resumed = stopped.recovered(stopped.revision, &catalog).unwrap();
    let claimed = resumed
        .worker(
            resumed.revision,
            200,
            WorkerCommand::Claim { duration: 100 },
            &catalog,
        )
        .unwrap();
    let current = claimed.execution.as_ref().unwrap();
    assert_eq!(current.package, old.package);
    assert_eq!(current.prefix, old.prefix);
    assert!(current.fence > old.fence);
    assert!(
        claimed
            .worker(
                claimed.revision,
                201,
                WorkerCommand::Seal {
                    fence: old.fence,
                    expected: old.prefix
                },
                &catalog
            )
            .is_err()
    );
    let sealed = claimed
        .worker(
            claimed.revision,
            201,
            WorkerCommand::Seal {
                fence: current.fence,
                expected: current.prefix,
            },
            &catalog,
        )
        .unwrap();
    let old_result = result(&sealed, Outcome::Victory);
    let quarantine = sealed.quarantined(sealed.revision, &catalog).unwrap();
    let recovered = quarantine.recovered(quarantine.revision, &catalog).unwrap();
    assert_eq!(recovered.phase, Phase::Verifying);
    assert!(
        recovered
            .settled(recovered.revision, old_result, &catalog, &SEED)
            .is_err()
    );
    let finished = recovered
        .settled(
            recovered.revision,
            result(&recovered, Outcome::Victory),
            &catalog,
            &SEED,
        )
        .unwrap();
    assert_eq!(finished.wins(), 1);
    let retired = stopped.interrupted(stopped.revision, &catalog).unwrap();
    assert_eq!(retired.phase, Phase::Interrupted);
    assert_eq!(retired.economy, initial.economy);
    assert!(retired.history.is_empty());
    assert!(retired.recovered(retired.revision, &catalog).is_err());
}

#[test]
fn guarantees_revision_exhaustion_and_forged_finances_fail_atomically() {
    let (catalog, mut run) = fixture();
    for _ in 0..6 {
        run = advance(&run, Outcome::Victory, &catalog);
    }
    assert!(run.shop.mythic_choice);
    let wrong = Command::Sell { item: u32::MAX };
    assert!(run.command(run.revision, &wrong, &catalog, &SEED).is_err());
    assert!(run.shop.mythic_choice);
    let edited = run
        .command(
            run.revision,
            &Command::Edit {
                deck: run.cards.deck.clone(),
            },
            &catalog,
            &SEED,
        )
        .unwrap();
    assert!(!edited.shop.mythic_choice);
    assert!(
        edited
            .command(
                edited.revision,
                &Command::ChooseMythic {
                    slot: 6,
                    replace_pinned: false
                },
                &catalog,
                &SEED
            )
            .is_err()
    );
    let mut invalid = run.clone();
    invalid.economy.gold += 1;
    assert!(invalid.validate(&catalog).is_err());
    invalid = run.clone();
    invalid.shop.stage -= 1;
    assert!(invalid.validate(&catalog).is_err());
    invalid = run.clone();
    invalid.ledger.resold = 1;
    invalid.economy.gold += 1;
    assert!(invalid.validate(&catalog).is_err());
    run.revision = u64::MAX;
    assert!(
        run.command(run.revision, &Command::Refresh, &catalog, &SEED)
            .is_err()
    );
    assert!(run.shop.mythic_choice);
    assert!(serde_json::from_str::<Command>(r#"{"kind":"sell","item":1,"price":999}"#).is_err());
    assert!(serde_json::from_str::<Command>(r#"{"kind":"victory"}"#).is_err());
}
