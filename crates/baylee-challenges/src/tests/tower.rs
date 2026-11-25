use super::*;
use crate::stock::{Error, Report};
use crate::tower::{Prepared, reconcile};
use central_api::challenges::{HostRequest, HostResponse};
use central_api::tower as api;

/// A complete expedition; each encounter differs by its title.
fn expedition(period: Period) -> Prepared {
    let package = package();
    let nodes: Vec<_> = (0..api::NODES)
        .map(|n| publish(&package, period, &format!("Encounter {n}")))
        .collect();
    let upgrade = || api::Upgrade {
        title: words("Sharpen"),
        summary: words("A better blade."),
        copies: 1,
        remove: "Plains".into(),
        add: "Island".into(),
    };
    let track = || [upgrade(), upgrade(), upgrade(), upgrade()];
    Prepared {
        manifest: api::Manifest {
            theme: api::Theme {
                period,
                title: words("The Spire"),
                floors: std::array::from_fn(|_| words("Floor")),
                upgrades: [track(), track(), track()],
            },
            nodes: nodes.iter().map(|node| node.digest).collect(),
        },
        nodes,
    }
}

fn week() -> Period {
    slot(Kind::Weekly, MONDAY)
}

fn shelf() -> api::Stock {
    api::Stock {
        id: id(1),
        period: week(),
        staged: Vec::new(),
        published: false,
        withdrawn: false,
    }
}

fn all_nodes() -> Vec<u8> {
    (0..u8::try_from(api::NODES).unwrap()).collect()
}

/// One pass whose inventory is `stock`; anything beyond reading it panics.
fn with_inventory(stock: Vec<api::Stock>) -> Result<Report, Error> {
    let mut stock = Some(stock);
    reconcile(
        MONDAY,
        1,
        |request| match request {
            HostRequest::Tower {
                request: api::HostRequest::Inventory { .. },
            } => Ok(HostResponse::Tower {
                stock: stock.take().expect("asked once"),
            }),
            _ => panic!("nothing may be staged from this inventory"),
        },
        |_| Err(Error::Uncertified),
    )
}

/// One pass over an authority that starts from `start` and distorts each
/// answer with `lie(step, answer)`; step 35 is the activation.
fn against(
    start: &[api::Stock],
    mut lie: impl FnMut(usize, &mut Vec<api::Stock>),
) -> (Result<Report, Error>, usize) {
    let mut shelf = start.first().cloned().unwrap_or_else(shelf);
    let mut step = 0;
    let result = reconcile(
        MONDAY,
        1,
        |request| {
            let HostRequest::Tower { request } = request else {
                panic!("an expedition is never sent as a daily task");
            };
            let mut stock = match request {
                api::HostRequest::Inventory { from, until } => {
                    assert_eq!((from, until), (MONDAY, MONDAY + 1));
                    return Ok(HostResponse::Tower {
                        stock: start.to_vec(),
                    });
                }
                api::HostRequest::Stage { node, .. } => {
                    assert_eq!(usize::from(node), step, "stages go in order");
                    if !shelf.staged.contains(&node) {
                        shelf.staged.push(node);
                    }
                    vec![shelf.clone()]
                }
                api::HostRequest::Activate { season } => {
                    assert_eq!(season, shelf.id);
                    assert_eq!(step, api::NODES, "only a complete graph goes live");
                    shelf.published = true;
                    vec![shelf.clone()]
                }
                api::HostRequest::Withdraw { .. } => panic!("never withdrawn by itself"),
            };
            lie(step, &mut stock);
            step += 1;
            Ok(HostResponse::Tower { stock })
        },
        |period| Ok(expedition(period)),
    );
    (result, step)
}

#[test]
fn a_complete_expedition_is_staged_in_order_and_then_activated() {
    let (result, steps) = against(&[], |_, _| ());
    assert_eq!(
        result.unwrap(),
        Report {
            present: 0,
            published: 1
        }
    );
    assert_eq!(steps, api::NODES + 1);
}

#[test]
fn an_interrupted_delivery_resumes_the_season_it_began() {
    let mut begun = shelf();
    begun.staged = vec![0, 1, 2];
    let (result, steps) = against(std::slice::from_ref(&begun), |_, _| ());
    assert_eq!(result.unwrap().published, 1);
    assert_eq!(steps, api::NODES + 1);
    // A second season under the same week would lose the staged work.
    let (result, steps) = against(&[begun], |_, stock| stock[0].id = id(9));
    assert!(matches!(result, Err(Error::Invalid)));
    assert_eq!(steps, 1);
}

#[test]
fn a_published_or_withdrawn_season_is_left_alone() {
    let mut live = shelf();
    live.published = true;
    live.staged = all_nodes();
    let mut gone = shelf();
    gone.withdrawn = true;
    for season in [live, gone] {
        let report = with_inventory(vec![season]).unwrap();
        assert_eq!((report.present, report.published), (1, 0));
    }
}

#[test]
fn a_stage_that_is_not_confirmed_stops_the_delivery() {
    let lies: [fn(&mut Vec<api::Stock>); 5] = [
        |stock| stock[0].withdrawn = true,
        |stock| {
            stock[0].staged.pop();
        },
        Vec::clear,
        |stock| stock[0].period = slot(Kind::Weekly, MONDAY + 7 * DAY),
        |stock| stock[0].id = id(9),
    ];
    for (at, lie) in lies.into_iter().enumerate() {
        let (result, steps) = against(&[], |step, stock| {
            if step == 4 {
                lie(stock);
            }
        });
        assert!(matches!(result, Err(Error::Invalid)), "{at}");
        assert_eq!(steps, 5, "{at}");
    }
}

#[test]
fn an_activation_that_is_not_confirmed_is_not_counted() {
    let lies: [fn(&mut Vec<api::Stock>); 4] = [
        |stock| stock[0].published = false,
        |stock| stock[0].withdrawn = true,
        |stock| stock[0].id = id(9),
        Vec::clear,
    ];
    for (at, lie) in lies.into_iter().enumerate() {
        let (result, steps) = against(&[], |step, stock| {
            if step == api::NODES {
                lie(stock);
            }
        });
        assert!(matches!(result, Err(Error::Invalid)), "{at}");
        assert_eq!(steps, api::NODES + 1, "{at}");
    }
}

#[test]
fn stock_that_cannot_be_true_is_refused_before_anything_is_generated() {
    // `Uncertified` is what the preparation answers, so this one was believed.
    assert!(matches!(
        with_inventory(vec![shelf()]),
        Err(Error::Uncertified)
    ));
    let lies: [fn(&mut api::Stock); 9] = [
        |s| s.period = slot(Kind::Daily, MONDAY),
        |s| s.period.ends_at += 1,
        |s| s.period = slot(Kind::Weekly, MONDAY - DAY),
        |s| s.period = slot(Kind::Weekly, MONDAY + 7 * DAY),
        |s| s.id[6] = 0x40,
        |s| s.id[8] = 0xC0,
        |s| s.staged = vec![35],
        |s| s.staged = vec![1, 1],
        |s| s.staged = vec![2, 1],
    ];
    for (at, lie) in lies.into_iter().enumerate() {
        let mut season = shelf();
        lie(&mut season);
        assert!(
            matches!(with_inventory(vec![season]), Err(Error::Invalid)),
            "{at}"
        );
    }
    let mut live = shelf();
    live.published = true;
    live.staged = all_nodes()[1..].to_vec();
    assert!(
        matches!(with_inventory(vec![live]), Err(Error::Invalid)),
        "published without its first encounter"
    );
}

#[test]
fn two_seasons_of_one_week_or_one_name_are_refused() {
    let mut other = shelf();
    other.id = id(2);
    let same_week = vec![shelf(), other];
    assert!(matches!(with_inventory(same_week), Err(Error::Invalid)));
    // Two weeks apart, so only the name is shared: asked over a longer horizon.
    let mut later = shelf();
    later.period = slot(Kind::Weekly, MONDAY + 7 * DAY);
    let pass = |stock: Vec<api::Stock>| {
        reconcile(
            MONDAY,
            14,
            |_| {
                Ok(HostResponse::Tower {
                    stock: stock.clone(),
                })
            },
            |_| Err(Error::Uncertified),
        )
    };
    assert!(matches!(
        pass(vec![shelf(), later.clone()]),
        Err(Error::Invalid)
    ));
    later.id = id(2);
    assert!(matches!(
        pass(vec![shelf(), later]),
        Err(Error::Uncertified)
    ));
}

#[test]
fn an_answer_that_is_not_about_the_expedition_is_refused() {
    let result = reconcile(
        MONDAY,
        1,
        |_| {
            Ok(HostResponse::Inventory {
                challenges: Vec::new(),
            })
        },
        |_| panic!("nothing may be generated"),
    );
    assert!(matches!(result, Err(Error::Invalid)));
}

#[test]
fn retained_stock_must_be_complete_untouched_and_for_its_own_week() {
    assert!(expedition(week()).validate(week()).is_ok());
    let next = slot(Kind::Weekly, MONDAY + 7 * DAY);
    assert!(matches!(
        expedition(week()).validate(next),
        Err(Error::Invalid)
    ));

    let mut short = expedition(week());
    short.nodes.pop();
    assert!(matches!(short.validate(week()), Err(Error::Invalid)));

    let mut swapped = expedition(week());
    swapped.nodes.swap(0, 34);
    assert!(matches!(swapped.validate(week()), Err(Error::Uncertified)));

    let mut doubled = expedition(week());
    doubled.manifest.nodes[1] = doubled.manifest.nodes[0];
    assert!(matches!(doubled.validate(week()), Err(Error::Uncertified)));

    // Certified and listed, but for a build this executor is not.
    let mut foreign = expedition(week());
    let mut package = package();
    package.executor = "another build".into();
    foreign.nodes[34] = publish(&package, week(), "Encounter 34");
    foreign.manifest.nodes[34] = foreign.nodes[34].digest;
    assert!(matches!(foreign.validate(week()), Err(Error::Uncertified)));
}

#[test]
fn a_bundle_that_fails_its_own_check_is_never_staged() {
    let result = reconcile(
        MONDAY,
        1,
        |request| match request {
            HostRequest::Tower {
                request: api::HostRequest::Inventory { .. },
            } => Ok(HostResponse::Tower { stock: Vec::new() }),
            _ => panic!("an incomplete graph may not be staged"),
        },
        |period| {
            let mut prepared = expedition(period);
            prepared.nodes.truncate(1);
            Ok(prepared)
        },
    );
    assert!(matches!(result, Err(Error::Invalid)));
}
