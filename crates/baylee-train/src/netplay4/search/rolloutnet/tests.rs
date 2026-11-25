//! A rollout net beside the searching one, with stand-ins for both: what
//! plays, what judges a leaf, and that nothing but the net changes.
use super::super::tests::{
    ME, Mock, SEED, config, flat, land_playable, lands, passes, root, table,
};
use super::super::{
    KnownDecks, PriorDecks, SearchConfig4, SearchOutcome4, SearchReport4, SearchedAnswer4,
    search_decision,
};
use super::*;
use baylee_gamehost::Session;

/// A net and its rollout net at the land-drop root of `session`, the net
/// asked `asked` times at the table before it. Both watched the root.
struct AtRoot {
    session: Session,
    view: PlayerView,
    pending: Pending,
}

impl AtRoot {
    fn new() -> Self {
        let mut session = table(25);
        let (pending, view) = root(&mut session, land_playable);
        Self {
            session,
            view,
            pending,
        }
    }

    /// `player` once it watched the root and decided `asked` questions there.
    fn seated(&self, player: Mock, asked: u64) -> Mock {
        let mut player = player.lending();
        player.observe(&self.view).unwrap();
        for _ in 0..asked {
            let decision = player.decide(&self.view, &self.pending, &[]);
            player.conclude(&self.view, &decision, &decision.answer.action);
        }
        player
    }

    fn search<P: SearchPlayer4 + Send>(
        &self,
        player: &mut P,
        config: SearchConfig4,
    ) -> anyhow::Result<SearchedAnswer4> {
        let (view, pending) = (&self.view, &self.pending);
        if config.fair() {
            let fair = PriorDecks::new(&self.session, view);
            return search_decision(player, &fair, view, pending, &[], config, SEED);
        }
        let known = KnownDecks(&self.session);
        search_decision(player, &known, view, pending, &[], config, SEED)
    }
}

fn searched(answer: &SearchedAnswer4) -> &SearchReport4 {
    let SearchOutcome4::Searched(report) = &answer.outcome else {
        panic!("the root is searched: {:?}", answer.outcome);
    };
    report
}

/// The searches a rollout net runs in: the settled stop, a turn horizon,
/// macro candidates in fair worlds, every chair played by the net, the
/// targets of a cast, and lockstep.
fn searches() -> Vec<SearchConfig4> {
    let settled = config(4, 3, 6);
    let turns = settled.with_turns(2).unwrap().with_fair(true);
    vec![
        settled,
        turns,
        turns.with_macros(true).with_neural_opponent(true),
        settled.with_targets(2).unwrap(),
        turns.with_lockstep(true),
    ]
}

/// With itself for a rollout net and its own value followed to the leaf, a
/// search is the search without a rollout net: the same candidates, worlds,
/// answer, and the same player afterwards. What the net noted at the table
/// is in the worlds, although the rollout net never answered there.
#[test]
fn the_net_as_its_own_rollout_net_with_follow_is_the_plain_search() {
    let at = AtRoot::new();
    let net = || {
        let mut net = Mock::new(passes, lands);
        net.passes_after = Some(4);
        net
    };
    for config in searches() {
        let mut plain = at.seated(net(), 3);
        let without = at.search(&mut plain, config).unwrap();

        let mut searching = at.seated(net(), 3);
        let mut rollout = at.seated(net(), 0);
        let mut both =
            RolloutNet4::new(&mut searching, &mut rollout, RolloutLeaf4::Follow).unwrap();
        let with = at.search(&mut both, config).unwrap();

        assert_eq!(with.outcome, without.outcome, "{config:?}");
        assert_eq!(with.answer.action, without.answer.action);
        assert_eq!(with.answer.provenance, without.answer.provenance);
        assert_eq!(with.answer.target, without.answer.target);
        assert_eq!(
            (&with.follow, &with.script),
            (&without.follow, &without.script)
        );
        assert_eq!(searching.state, plain.state, "{config:?}");
        assert_eq!(rollout.state.observations, 1, "it only watched the root");

        // The notes are what the worlds turn on here: a net that was never
        // asked at the table rolls the same candidates out otherwise.
        let mut unasked = at.seated(net(), 0);
        let other = at.search(&mut unasked, config).unwrap();
        assert_ne!(
            searched(&other).worlds,
            searched(&without).worlds,
            "{config:?}"
        );
    }
}

/// Each leaf is judged by whom it names: the rollout net's own value under
/// `student`, the searching net's under `jump` and `follow`.
#[test]
fn the_leaf_names_whose_value_a_rollout_is_scored_by() {
    let at = AtRoot::new();
    for (leaf, judged) in [
        (RolloutLeaf4::Student, 0.75),
        (RolloutLeaf4::Jump, 0.25),
        (RolloutLeaf4::Follow, 0.25),
    ] {
        let mut net = at.seated(Mock::new(passes, |_| 0.25), 0);
        let mut rollout = at.seated(Mock::new(passes, |_| 0.75), 0);
        let mut both = RolloutNet4::new(&mut net, &mut rollout, leaf).unwrap();
        let answer = at.search(&mut both, config(3, 2, 2)).unwrap();
        let report = searched(&answer);
        assert!(report.rollout_steps > 0, "the worlds are played on");
        assert_eq!(report.leaves.terminal + report.leaves.taken_back, 0);
        assert!(
            report
                .worlds
                .iter()
                .flatten()
                .all(|score| score.to_bits() == f32::to_bits(judged)),
            "{leaf:?}: {report:?}"
        );
        assert_eq!(answer.answer.value.to_bits(), 0.5_f32.to_bits());
    }
}

/// What the judge of a leaf sees of a rollout: every frame of the seat under
/// `follow`, the last one alone under `jump`, and never another seat's.
#[test]
fn the_judge_sees_the_frames_its_leaf_says() {
    let at = AtRoot::new();
    let theirs = at
        .session
        .observe_view_with_context(baylee_core::ids::PlayerId::new(1));
    for (leaf, playing, judging) in [(RolloutLeaf4::Jump, 0, 1), (RolloutLeaf4::Follow, 2, 2)] {
        let mut net = at.seated(Mock::new(passes, flat), 0);
        let mut rollout = at.seated(Mock::new(passes, flat), 0);
        let both = RolloutNet4::new(&mut net, &mut rollout, leaf).unwrap();
        let before = both.net().state.observations;
        let mut player = both.rollout_player(&both.snapshot()).unwrap();
        player.reset_other_seats_for_rollout(ME);
        player.observe(&at.view).unwrap();
        player.observe(&theirs).unwrap();
        player.observe(&at.view).unwrap();
        assert_eq!(
            player.net().state.observations - before,
            playing,
            "{leaf:?}"
        );
        assert!(player.leaf_value(ME).is_some());
        assert_eq!(
            player.net().state.observations - before,
            judging,
            "{leaf:?}"
        );
        assert_eq!(player.rollout.get().state.observations, 4, "{leaf:?}");
    }
    let mut net = at.seated(Mock::new(passes, flat), 0);
    let mut rollout = at.seated(Mock::new(passes, flat), 0);
    let both = RolloutNet4::new(&mut net, &mut rollout, RolloutLeaf4::Student).unwrap();
    let player = both.rollout_player(&both.snapshot()).unwrap();
    assert!(player.net.is_none(), "its own value needs no judge");
}

/// A rollout net that lends no player, or that reads other views, is refused
/// rather than played by the searching net.
#[test]
fn a_rollout_net_that_cannot_play_the_worlds_is_refused() {
    let at = AtRoot::new();
    let mut net = at.seated(Mock::new(passes, flat), 0);
    let mut alone = Mock::new(passes, flat);
    alone.observe(&at.view).unwrap();
    let mut both = RolloutNet4::new(&mut net, &mut alone, RolloutLeaf4::Follow).unwrap();
    let error = at.search(&mut both, config(3, 1, 2)).unwrap_err();
    assert!(
        error.to_string().contains("lends no rollout player"),
        "{error:#}"
    );

    let mut other = Mock::new(passes, flat);
    other.context = ViewContext4::Plain;
    let error = RolloutNet4::new(&mut net, &mut other, RolloutLeaf4::Student)
        .err()
        .unwrap();
    assert!(error.to_string().contains("other views"), "{error:#}");
}

#[test]
fn a_leaf_is_read_by_its_name() {
    for leaf in RolloutLeaf4::ALL {
        assert_eq!(leaf.name().parse::<RolloutLeaf4>().unwrap(), leaf);
    }
    assert!("value".parse::<RolloutLeaf4>().is_err());
}
