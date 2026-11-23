//! Kill the actual primary after a jointly certified decision. The promoted engine
//! must restore both private seats before a third independent engine certifies the result.
use super::{
    AccountStore, Assignment, DatabaseConnection, Format, Id, Outcome, Player, QueueState,
    Registered, Request, Response, Runtime, Served, common, consent, fresh, player, receipts,
    registered,
};
use baylee_account::ranked::replica::{Phase, SeatStatus};
use baylee_engine::choice::PlayerAction;

#[path = "replication/authority.rs"]
mod authority;
#[path = "replication/host.rs"]
mod host;
#[path = "replication/seats.rs"]
mod seats;
#[path = "replication/witnesses.rs"]
mod witnesses;
pub(super) use authority::join;
use host::Host;
use seats::Connected;

#[test]
#[ignore = "requires isolated PostgreSQL 18, clean gateway/agent/engine binaries, and Central"]
fn mirrored_duel_recovers_two_private_seats_after_primary_process_loss() {
    super::super::integration(super::super::Scenario::Replicated);
}

struct Fixture<'a> {
    rt: &'a Runtime,
    served: &'a Served,
    store: &'a AccountStore,
    db: &'a DatabaseConnection,
}

impl Fixture<'_> {
    fn register(&self, name: &str) -> baylee_account::Account {
        fresh::register(
            self.rt,
            &self.served.service,
            &self.served.endpoint,
            name,
            fresh::DOCUMENT,
        )
    }

    fn gateway(&self, owner: &str, verified: bool) -> Registered {
        registered(self.rt, self.served, self.store, self.db, owner, verified)
    }

    fn host(&self, owner: &str) -> Host {
        Host::start(self.rt, self.db, self.gateway(owner, true))
    }
}

struct Saved {
    route: SeatStatus,
    views: [baylee_view::PlayerView; 2],
    live: Connected,
}

fn advance(f: &Fixture<'_>, players: &[Player; 2], host: &Registered, match_id: Id) -> Saved {
    let rt = f.rt;
    super::relay::admissions(rt, f.served, host, players, match_id);
    // Both participants deliver their frozen decks to the verified primary.
    let tickets = seats::bind(players, match_id, true);
    let opening = authority::wait(&players[0], match_id, |s| s.phase == Phase::Playing);
    assert_eq!(opening.fence.generation, 1);
    assert_eq!(opening.game, Some(match_id));
    assert_eq!(opening.head.position, 0);
    let mut sockets = Connected::new(f, host, &tickets, players, &opening);
    let views = sockets.views(rt, &opening);
    super::relay::verify_peers(f.served, host);
    rt.block_on(super::relay::reject(
        f.served,
        tickets[0].seat.seat_token.as_bytes(),
    ));
    rt.block_on(super::relay::reject(f.served, &[b'0'; 64]));
    assert!(views[0].deciding.contains(views[0].seat));
    let keep = authority::intent(&opening, &PlayerAction::MulliganKeep);
    authority::submit(&players[0], &keep);
    authority::submit(&players[0], &keep);
    let route = authority::wait(&players[1], match_id, |s| s.head.position == 1);
    assert_ne!(route.head.chain, opening.head.chain);
    drop(sockets);
    let tickets = seats::bind(players, match_id, false);
    let mut live = Connected::new(f, host, &tickets, players, &route);
    let current = live.views(rt, &route);
    assert!(!current[0].deciding.contains(current[0].seat));
    assert!(current[1].deciding.contains(current[1].seat));
    Saved {
        route,
        views: current,
        live,
    }
}

fn home_identity(home: &Registered, players: &[Player; 2]) -> [String; 2] {
    players.each_ref().map(|p| {
        let (code, body) = common::http(home.gateway.port, "GET", "/me", Some(&p.home.token), "");
        assert_eq!(code, 200, "original lobby bearer must still be usable");
        common::json_field(&body, "id").to_owned()
    })
}

/// Admit a fresh independent operator after the existing standby was promoted.
/// Its recovery cannot expose a journal prefix before both players consent.
fn replace(
    f: &Fixture<'_>,
    players: &[Player; 2],
    leader: &Registered,
    saved: &Saved,
    seed: u64,
) -> (Host, SeatStatus) {
    let match_id = saved.route.fence.match_id;
    let replacement = f.host("mirror_three");
    let pair = [leader, &replacement.registered];
    consent::approve(players, match_id, &pair, |p| {
        consent::no_private_work(f.served, &replacement.registered, &p[0], match_id);
        assert_eq!(authority::status(&p[0], match_id).head, saved.route.head);
    });
    let recovered = authority::wait(&players[0], match_id, |s| s.phase == Phase::Playing);
    assert_eq!(recovered.host_id, leader.id);
    assert_eq!(recovered.head, saved.route.head);
    assert_eq!(
        seed,
        witnesses::assignments(f.served, players, pair, match_id)
    );
    witnesses::quorum(f.rt, f.db, &recovered, pair.map(|h| h.id));
    (replacement, recovered)
}

fn finish(
    f: &Fixture<'_>,
    players: &[Player; 2],
    leader: &Registered,
    saved: &Saved,
    recovered: &SeatStatus,
    assignments: &[Assignment; 2],
) {
    let match_id = saved.route.fence.match_id;
    super::relay::admissions(f.rt, f.served, leader, players, match_id);
    let tickets = seats::bind(players, match_id, false);
    for (ticket, assigned) in tickets.iter().zip(assignments) {
        assert_eq!(ticket.assignment.seat, assigned.seat);
        assert_eq!(ticket.assignment.account, assigned.account);
    }
    let mut sockets = Connected::new(f, leader, &tickets, players, recovered);
    let views = sockets.views(f.rt, recovered);
    super::relay::verify_peers(f.served, leader);
    for (before, after) in saved.views.iter().zip(&views) {
        seats::same_view(before, after);
    }
    let concede = authority::intent(recovered, &PlayerAction::Concede);
    authority::submit(&players[0], &concede);
    let terminal = authority::wait(&players[1], match_id, |s| s.phase == Phase::Finished);
    assert_eq!(terminal.head.position, recovered.head.position + 1);
    sockets.terminal(f.rt, &terminal);
    // Losing the acknowledgement cannot turn one confirmed result into two.
    let _ = players[0].account.replica(concede);
    let outcome = if assignments[0].seat == 0 {
        Outcome::SecondWins
    } else {
        Outcome::FirstWins
    };
    receipts::settled(f.served, players, assignments, outcome);
}

pub(in super::super) fn run(
    rt: &Runtime,
    served: &Served,
    store: &AccountStore,
    db: &DatabaseConnection,
) {
    let fixture = Fixture {
        rt,
        served,
        store,
        db,
    };
    let first = fresh::register_as(
        rt,
        &served.service,
        &served.endpoint,
        "mirror_one",
        None,
        fresh::DOCUMENT,
    );
    let second = fixture.register("mirror_two");
    let _home_owner = fixture.register("mirror_home");
    let _replacement_owner = fixture.register("mirror_three");
    let home = fixture.gateway("mirror_home", false);
    let players = [
        player(first, served, &home, 60),
        player(second, served, &home, 61),
    ];
    receipts::before(served, &players);
    let identities = home_identity(&home, &players);
    let mut hosts = [fixture.host("mirror_one"), fixture.host("mirror_two")];
    let assignments = authority::join(&players);
    let match_id = assignments[0].match_id;
    assert_eq!(assignments[1].match_id, match_id);
    assert_ne!(assignments[0].seat, assignments[1].seat);
    let first_route = authority::status(&players[0], match_id);
    assert_eq!(first_route.phase, Phase::Preparing);
    let primary = hosts
        .iter()
        .position(|h| h.registered.id == first_route.host_id)
        .expect("real primary");
    let standby = 1 - primary;
    consent::approve(
        &players,
        match_id,
        &[&hosts[primary].registered, &hosts[standby].registered],
        |p| {
            consent::unopened(rt, db, match_id);
            consent::no_private_work(served, &hosts[standby].registered, &p[0], match_id);
        },
    );
    let seed = witnesses::assignments(
        served,
        &players,
        [&hosts[primary].registered, &hosts[standby].registered],
        match_id,
    );
    let mut saved = advance(&fixture, &players, &hosts[primary].registered, match_id);
    witnesses::quorum(
        rt,
        db,
        &saved.route,
        hosts.each_ref().map(|h| h.registered.id),
    );
    hosts[primary].crash_gateway();
    saved.live.disconnected(rt);
    let alone = authority::wait(&players[1], match_id, |s| {
        s.fence.generation > saved.route.fence.generation
    });
    assert_eq!(alone.host_id, hosts[standby].registered.id);
    assert_eq!(alone.game, saved.route.game);
    assert_eq!(alone.head, saved.route.head);
    assert_eq!(
        alone.phase,
        Phase::Paused,
        "a single survivor cannot certify new input"
    );
    assert!(
        players[0]
            .account
            .replica(authority::intent(&saved.route, &PlayerAction::Concede))
            .is_err()
    );
    assert_eq!(identities, home_identity(&home, &players));
    let (_replacement, recovered) =
        replace(&fixture, &players, &hosts[standby].registered, &saved, seed);
    finish(
        &fixture,
        &players,
        &hosts[standby].registered,
        &saved,
        &recovered,
        &assignments,
    );
    assert_eq!(identities, home_identity(&home, &players));
}
