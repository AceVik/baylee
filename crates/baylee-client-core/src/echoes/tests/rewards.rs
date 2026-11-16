use super::*;
use baylee_core::color::{Color, ColorSet};
use domain::cards::{Formation, RunCard};

pub(super) fn fixture() -> (State, Cohort, Run) {
    let mut cards: Vec<_> = Color::ALL
        .into_iter()
        .enumerate()
        .map(|(index, color)| Card {
            card: CardIndex::new(u32::try_from(index).unwrap()),
            price: 0,
            basic_land: true,
            run: Some(RunCard {
                presentation: None,
                identity: ColorSet::of(color),
                commander: false,
            }),
        })
        .collect();
    cards.extend((5..10).map(|id| Card {
        card: CardIndex::new(id),
        price: 4,
        basic_land: false,
        run: Some(RunCard {
            presentation: None,
            identity: ColorSet::of(Color::Green),
            commander: true,
        }),
    }));
    let catalog = Catalog { cards };
    let starters: Vec<_> = (5..10)
        .map(|id| Collection {
            formation: Some(Formation {
                commander: CardIndex::new(id),
                sideboard: vec![],
            }),
            owned: vec![Stack {
                card: CardIndex::new(id),
                copies: 1,
            }],
            deck: vec![Stack {
                card: CardIndex::new(4),
                copies: 39,
            }],
        })
        .collect();
    let mut executor = cohort().contract.executor;
    executor.rules = "tower-5".into();
    executor.ai = "tower-house-22".into();
    let cohort = Cohort {
        id: Uuid::now_v7(),
        contract: domain::v2::package::Cohort::for_generation(
            executor,
            &catalog,
            domain::v2::generation::RUN,
        )
        .unwrap(),
        catalog,
        starters,
        offered: vec![0, 1, 2, 3, 4],
    };
    let raw = domain::v2::run::Run::begin_for(
        &cohort.catalog,
        cohort.starters[0].clone(),
        &[7; 32],
        domain::v2::generation::RUN,
    )
    .unwrap();
    let mut run = Run::project(Uuid::now_v7(), cohort.id, raw);
    // Synthetic wire fixture; authority replay/settlement is exercised separately.
    run.phase = Phase::Reward;
    run.history.push(api::Duel {
        outcome: domain::run::Outcome::Victory,
        turns: 12,
        used: [0; 2],
        bonus_gold: 0,
        claim: None,
    });
    run.reward = Some(api::Reward {
        cards: (5..10).map(CardIndex::new).collect(),
        take: 3,
    });
    let mut state = State::default();
    let work = state.read(Uuid::now_v7(), Read::Cohorts(None)).unwrap();
    state.receive(
        &work,
        Some(Response::Cohorts {
            consent: 1,
            cohorts: vec![cohort.clone()],
        }),
        None,
    );
    let work = state.read(Uuid::now_v7(), Read::Run(run.id)).unwrap();
    state.receive(
        &work,
        Some(Response::Run {
            run: Box::new(run.clone()),
        }),
        None,
    );
    assert_eq!(state.run(), Some(&run));
    (state, cohort, run)
}

#[test]
fn runic_shops_accept_twelve_and_refuse_wrong_size_or_unknown_generation() {
    let (mut state, cohort, _) = fixture();
    let raw = domain::v2::run::Run::begin_for(
        &cohort.catalog,
        cohort.starters[0].clone(),
        &[4; 32],
        domain::v2::generation::RUNIC,
    )
    .unwrap();
    let run = Run::project(Uuid::now_v7(), cohort.id, raw);
    assert_eq!(run.shop.offers.len(), 12);
    let work = state.read(Uuid::now_v7(), Read::Run(run.id)).unwrap();
    state.receive(
        &work,
        Some(Response::Run {
            run: Box::new(run.clone()),
        }),
        None,
    );
    assert_eq!(state.run(), Some(&run));
    let mut missing = run.clone();
    missing.revision += 1;
    missing.shop.offers.pop();
    assert!(!state.push(Response::Run {
        run: Box::new(missing)
    }));
    let mut unknown = run.clone();
    unknown.revision += 1;
    unknown.shop.generator = 999;
    assert!(!state.push(Response::Run {
        run: Box::new(unknown)
    }));
    assert_eq!(state.run(), Some(&run));
}

#[test]
fn new_run_cards_select_exactly_three_and_never_change_on_read_refresh() {
    let (_, cohort, run) = fixture();
    let mut draft = draft::Draft::default();
    draft.bind(&run);
    for id in [999, 5, 6, 7, 8] {
        draft.reward(&run, CardIndex::new(id));
    }
    assert_eq!(draft.rewards, [5, 6, 7].map(CardIndex::new));
    draft.bind(&run);
    assert_eq!(draft.rewards.len(), 3);
    assert!(draft::allowed(
        &run,
        &cohort,
        &Command::Claim {
            cards: draft.rewards.clone()
        }
    ));
    assert!(!draft::allowed(
        &run,
        &cohort,
        &Command::Claim {
            cards: vec![CardIndex::new(5); 3]
        }
    ));
    assert!(!draft::allowed(&run, &cohort, &Command::Refresh {}));
    draft.reward(&run, CardIndex::new(6));
    assert!(!draft::allowed(
        &run,
        &cohort,
        &Command::Claim {
            cards: draft.rewards.clone()
        }
    ));
    let mut newer = run;
    newer.revision += 1;
    draft.bind(&newer);
    assert!(draft.rewards.is_empty());
}

#[test]
fn client_refuses_forged_reward_cardinality_and_fifth_starter_is_reachable() {
    let (mut state, cohort, run) = fixture();
    let mut forged = run.clone();
    forged.reward.as_mut().unwrap().take = 4;
    let work = state.read(Uuid::now_v7(), Read::Run(run.id)).unwrap();
    state.receive(
        &work,
        Some(Response::Run {
            run: Box::new(forged),
        }),
        None,
    );
    assert_eq!(state.run(), Some(&run));
    assert_eq!(state.failure(), Some(Error::Unavailable));
    assert!(state.start(Uuid::now_v7(), cohort.id, 5, 1).is_none());
    let work = state.start(Uuid::now_v7(), cohort.id, 4, 1).unwrap();
    assert!(matches!(
        work.request,
        Some(Request::Start { starter: 4, .. })
    ));
}
