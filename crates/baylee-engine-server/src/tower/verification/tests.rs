use super::*;
use baylee_core::{ids::PlayerId, preset::TowerSupply};
use baylee_engine::{
    choice::{Pending, PlayerAction},
    zone::ZoneLocation,
};
use baylee_tower::{
    cards::{Card, Catalog, Stack},
    v2::{
        catalog::{Definition, Family, Rarity},
        execution::{Execution, MAX_RECORD_BYTES},
        package::{Build, Cohort, Origin},
    },
};
use std::{io::Cursor, time::Duration};

const ME: PlayerId = PlayerId::new(0);

fn package() -> (Package, Catalog) {
    let card = baylee_cards::decks::by_name("Forest").unwrap();
    let catalog = Catalog {
        cards: vec![Card {
            card,
            price: 0,
            basic_land: true,
        }],
    };
    let build = Build {
        deck: vec![Stack { card, copies: 40 }],
        active: vec![Definition {
            family: Family::Grove,
            rarity: Rarity::Common,
        }],
        potions: [1, 1],
    };
    let package = Package {
        schema: 2,
        cohort: Cohort::new(crate::variant::identity(), &catalog).unwrap(),
        stage: 1,
        opponent: Origin::House { archetype: 1 },
        builds: [build.clone(), build],
        shuffle_seed: 19,
        policy_seed: 17,
    };
    (package, catalog)
}

fn recorded(package: &Package, catalog: &Catalog, spend: bool) -> Vec<u8> {
    let identity = crate::variant::identity();
    let preset = package
        .preset(catalog, &identity, |card| {
            Some(baylee_cards::decks::reference_print(card))
        })
        .unwrap();
    let mut live =
        crate::variant::HostedSession::recorded_for_execution(&preset, &identity).unwrap();
    for _ in 0..32 {
        if live.needs_pump() {
            live.pump_slice(1);
        } else if matches!(live.pending(), Pending::Priority { player, .. } if *player == ME) {
            break;
        } else {
            live.act_slice(ME, live.house_action(ME).unwrap(), 0)
                .unwrap();
        }
    }
    assert!(matches!(live.pending(), Pending::Priority { player, .. } if *player == ME));
    if spend {
        for supply in [TowerSupply::HealingPotion, TowerSupply::ManaPotion] {
            let definition = baylee_cards::tokens::tower::definition(supply);
            let source = *live
                .state()
                .zones
                .list(ZoneLocation::Command(ME))
                .iter()
                .find(|id| {
                    live.state()
                        .object(**id)
                        .and_then(|object| object.token)
                        .is_some_and(|token| std::ptr::eq(token, definition))
                })
                .unwrap();
            live.act_slice(
                ME,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0,
                },
                0,
            )
            .unwrap();
        }
        live.act_slice(
            ME,
            PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Blue),
            0,
        )
        .unwrap();
    }
    live.act_slice(ME, PlayerAction::Concede, 0).unwrap();
    live.take_record()
}

fn framed(package: &Package, catalog: &Catalog, bytes: &[u8], width: usize) -> (Binding, Vec<u8>) {
    let mut execution = Execution::new(package.digest(catalog).unwrap(), MAX_RECORD_BYTES)
        .unwrap()
        .claimed(0, 1)
        .unwrap();
    let mut wire = Vec::new();
    for chunk in bytes.chunks(width) {
        baylee_worker::frame::write(
            &mut wire,
            chunk,
            baylee_tower::v2::execution::MAX_CHUNK_BYTES,
        )
        .unwrap();
        execution = execution.appended(1, 0, execution.prefix, chunk).unwrap();
    }
    (
        Binding {
            package: execution.package,
            fence: 7,
            prefix: execution.prefix,
        },
        wire,
    )
}

fn check(
    package: &Package,
    catalog: &Catalog,
    binding: Binding,
    wire: &[u8],
) -> Result<Settlement, Refused> {
    verify_record(
        package,
        catalog,
        binding,
        Cursor::new(wire),
        ReplayBudget::default(),
        Instant::now() + Duration::from_secs(10),
    )
}

#[test]
fn native_tower_adjudication_recomputes_expert_and_exact_charge_consumption() {
    let (package, catalog) = package();
    for spend in [false, true] {
        let bytes = recorded(&package, &catalog, spend);
        // Transport boundaries deliberately split JSON lines and UTF-8 independently.
        let (binding, wire) = framed(&package, &catalog, &bytes, 127);
        let result = check(&package, &catalog, binding, &wire).unwrap();
        assert_eq!(result.outcome, Outcome::Defeat);
        assert_eq!(result.used, [u8::from(spend); 2]);
        assert_eq!(result.binding, binding);
        assert!(result.turns > 0);
        let (other_binding, other_wire) = framed(&package, &catalog, &bytes, 2048);
        assert_ne!(other_binding.prefix.digest, binding.prefix.digest);
        assert_eq!(
            check(&package, &catalog, other_binding, &other_wire)
                .unwrap()
                .used,
            result.used
        );
        assert_eq!(
            check(&package, &catalog, binding, &other_wire),
            Err(Refused::Invalid)
        );
    }
}

#[test]
fn native_verifier_rejects_altered_artifact_ai_policy_result_and_chunk_stream() {
    let (package, catalog) = package();
    let bytes = recorded(&package, &catalog, false);
    let (binding, wire) = framed(&package, &catalog, &bytes, 2048);
    let mut wrong = package.clone();
    wrong.policy_seed += 1;
    assert_eq!(
        check(&wrong, &catalog, binding, &wire),
        Err(Refused::Invalid)
    );
    let mut extra = wire.clone();
    extra.push(0);
    assert_eq!(
        check(&package, &catalog, binding, &extra),
        Err(Refused::Invalid)
    );
    assert_eq!(
        check(&package, &catalog, binding, &wire[..wire.len() - 1]),
        Err(Refused::Invalid)
    );
    let mut huge = wire.clone();
    huge[..4].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(
        check(&package, &catalog, binding, &huge),
        Err(Refused::Invalid)
    );
    let mutations: [fn(&mut [baylee_gamehost::record::Line]); 4] = [
        |lines| {
            if let baylee_gamehost::record::Line::Header { execution, .. } = &mut lines[0] {
                *execution = None;
            }
        },
        |lines| {
            if let baylee_gamehost::record::Line::Input { hash, .. } = &mut lines[1] {
                *hash = "0000000000000000".into();
            }
        },
        |lines| {
            if let Some(baylee_gamehost::record::Line::Input { action, .. }) =
                lines.iter_mut().find(|line| {
                    matches!(
                        line,
                        baylee_gamehost::record::Line::Input {
                            by: baylee_gamehost::record::Source::House,
                            ..
                        }
                    )
                })
            {
                *action = PlayerAction::Concede;
            }
        },
        |lines| {
            if let Some(baylee_gamehost::record::Line::End { winners, .. }) = lines.last_mut() {
                *winners = vec![0];
            }
        },
    ];
    for mutate in mutations {
        let mut lines: Vec<baylee_gamehost::record::Line> = bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect();
        mutate(&mut lines);
        let mut forged = Vec::new();
        for line in lines {
            serde_json::to_writer(&mut forged, &line).unwrap();
            forged.push(b'\n');
        }
        assert_ne!(forged, bytes);
        // Even a freshly committed forged stream must still pass actual rules/AI replay.
        let (forged_binding, forged_wire) = framed(&package, &catalog, &forged, 2048);
        assert_eq!(
            check(&package, &catalog, forged_binding, &forged_wire),
            Err(Refused::Invalid)
        );
    }
}

#[test]
fn verifier_budget_and_artifact_skew_refuse_before_reading_a_record() {
    struct NoRead;
    impl std::io::Read for NoRead {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            panic!("incompatible work must be refused before reading private bytes");
        }
    }
    let (mut package, catalog) = package();
    let (mut binding, _) = framed(&package, &catalog, b"synthetic\n", 32);
    let resources = ReplayBudget {
        bytes: 1,
        ..ReplayBudget::default()
    };
    assert_eq!(
        verify_record(
            &package,
            &catalog,
            binding,
            NoRead,
            resources,
            Instant::now() + Duration::from_secs(10)
        ),
        Err(Refused::Budget)
    );
    package.cohort.executor.features = "0".repeat(64);
    binding.package = package.digest(&catalog).unwrap();
    assert_eq!(
        verify_record(
            &package,
            &catalog,
            binding,
            NoRead,
            ReplayBudget::default(),
            Instant::now() + Duration::from_secs(10)
        ),
        Err(Refused::Invalid)
    );
}

#[test]
fn private_worker_headers_are_bounded_strict_and_cannot_select_remote_code() {
    use baylee_tower::v2::verification::{
        MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, Rejected, Request, Resources, Response,
    };
    let (package, catalog) = package();
    let (binding, _) = framed(&package, &catalog, b"synthetic\n", 32);
    let request = Request {
        schema: 2,
        package,
        catalog,
        binding,
        resources: Resources::default(),
    };
    assert!(Request::decode(&request.encode().unwrap()).is_ok());
    assert!(Request::decode(&vec![b' '; MAX_REQUEST_BYTES + 1]).is_err());
    let mut value = serde_json::to_value(&request).unwrap();
    value["executable"] = "/tmp/untrusted-program".into();
    let mut input = Vec::new();
    baylee_worker::frame::write(
        &mut input,
        &serde_json::to_vec(&value).unwrap(),
        MAX_REQUEST_BYTES,
    )
    .unwrap();
    let mut output = Vec::new();
    super::worker::run(&mut input.as_slice(), &mut output).unwrap();
    let response: Response = serde_json::from_slice(
        &baylee_worker::frame::read(&mut output.as_slice(), MAX_RESPONSE_BYTES).unwrap(),
    )
    .unwrap();
    assert_eq!(
        response,
        Response::Rejected {
            reason: Rejected::Invalid
        }
    );
    let mut request = request;
    request.resources.bytes = 1;
    assert_eq!(request.validate(), Err(baylee_tower::Refusal::Capacity));
}
