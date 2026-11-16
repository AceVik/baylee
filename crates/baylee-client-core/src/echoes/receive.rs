use super::{Error, Phase, Request, Response, Run, State, Work, api, mutation, uuid7};

impl State {
    /// Whether a pushed snapshot changes the visible selection. Inspect this with
    /// a shared borrow before mutating a retained UI resource. A push never ACKs
    /// a pending mutation and cannot rewind its selected run.
    #[must_use]
    pub fn push_changes(&self, response: &Response) -> bool {
        match response {
            Response::Run { run } => {
                self.selected == Some(run.id)
                    && valid_run(run)
                    && self
                        .run
                        .as_ref()
                        .is_some_and(|old| old.revision < run.revision)
            }
            Response::Ranking { board } => {
                board.valid()
                    && self.ranking.as_ref().is_some_and(|old| {
                        old.cohort == board.cohort
                            && old.lane == board.lane
                            && old.listing.revision <= board.listing.revision
                            && old != board
                    })
            }
            Response::Cohorts { consent, cohorts } => {
                *consent > 0
                    && cohorts.len() <= api::PAGE_SIZE
                    && cohorts.iter().all(valid_cohort)
                    && (self.consent != *consent || self.cohorts != *cohorts)
            }
            Response::Hosts { run, hosts } => {
                self.selected == Some(*run) && hosts.len() <= api::PAGE_SIZE && self.hosts != *hosts
            }
            _ => false,
        }
    }

    /// Install a validated unsolicited snapshot without touching work/receipt state.
    /// Returns false for an unchanged, stale or unrelated projection.
    pub fn push(&mut self, response: Response) -> bool {
        if !self.push_changes(&response) {
            return false;
        }
        match response {
            Response::Run { run } => self.install(*run),
            Response::Ranking { board } => self.ranking = Some(board),
            Response::Cohorts { consent, cohorts } => {
                self.consent = consent;
                self.cohorts = cohorts;
            }
            Response::Hosts { hosts, .. } => self.hosts = hosts,
            _ => return false,
        }
        true
    }

    /// Complete only the current work. A valid mutation ACK resolves its identity
    /// even if the following seat admission failed; a read never resolves an
    /// ambiguous mutation. Historical retry receipts cannot rewind a newer view.
    /// Returns false for an obsolete worker response without changing any state.
    pub fn receive(
        &mut self,
        work: &Work,
        response: Option<Response>,
        failure: Option<Error>,
    ) -> bool {
        if self.active.as_ref() != Some(work) {
            return false;
        }
        self.active = None;
        self.failure = failure;
        let acknowledged = match (&work.request, response) {
            (Some(request), Some(response)) => self.accept(request, response),
            (None, None) => true,
            _ => false,
        };
        if work.request.as_ref().is_some_and(mutation)
            && (acknowledged || failure == Some(Error::Conflict))
        {
            self.pending = None;
        }
        if !acknowledged && failure.is_none() {
            self.failure = Some(Error::Unavailable);
        }
        true
    }

    fn accept(&mut self, request: &Request, response: Response) -> bool {
        match (request, response) {
            (Request::Ranking { cohort, lane }, Response::Ranking { board })
                if board.cohort == *cohort && board.lane == *lane && board.valid() =>
            {
                if self
                    .ranking
                    .as_ref()
                    .is_none_or(|old| old.listing.revision <= board.listing.revision)
                {
                    self.ranking = Some(board);
                }
            }
            (Request::Listing { revision, listed }, Response::Listing { listing })
                if revision.checked_add(1) == Some(listing.revision)
                    && listing.listed == *listed =>
            {
                // Do not leave an old public/private projection on screen after a toggle.
                // The following cohort read obtains scores, names and ranks atomically.
                self.ranking = None;
            }
            (Request::WithdrawEchoes {}, Response::Withdrawn {}) => {}
            (Request::Cohort { cohort: id }, Response::Cohort { cohort })
                if cohort.id == *id && valid_cohort(&cohort) =>
            {
                self.publication = Some(*cohort);
            }
            (Request::Cohorts { .. }, Response::Cohorts { consent, cohorts })
                if consent > 0
                    && cohorts.len() <= api::PAGE_SIZE
                    && cohorts.iter().all(valid_cohort) =>
            {
                self.cohorts = cohorts;
                self.consent = consent;
            }
            (Request::Runs { .. }, Response::Runs { mut runs })
                if runs.len() <= api::PAGE_SIZE && runs.iter().all(valid_run) =>
            {
                for run in &mut runs {
                    if let Some(newer) = self
                        .runs
                        .iter()
                        .chain(self.run.iter())
                        .find(|old| old.id == run.id && old.revision > run.revision)
                    {
                        *run = newer.clone();
                    }
                }
                let selected = runs
                    .iter()
                    .find(|run| Some(run.id) == self.selected)
                    .cloned();
                self.runs = runs;
                if let Some(run) = selected {
                    self.install(run);
                }
            }
            (Request::Start { cohort, .. }, Response::Run { run })
                if run.cohort == *cohort && valid_run(&run) =>
            {
                self.selected = Some(run.id);
                self.install(*run);
            }
            (
                Request::Run { run: id }
                | Request::Command { run: id, .. }
                | Request::Reserve { run: id, .. },
                Response::Run { run },
            ) if run.id == *id && valid_run(&run) => self.install(*run),
            (Request::Hosts { run: id, .. }, Response::Hosts { run, hosts })
                if run == *id && Some(run) == self.selected && hosts.len() <= api::PAGE_SIZE =>
            {
                self.hosts = hosts;
            }
            (Request::Locate { run }, Response::Location { location })
                if *run == location.run && Some(*run) == self.selected =>
            {
                if self
                    .run
                    .as_ref()
                    .is_none_or(|run| location.revision >= run.revision)
                {
                    self.location = Some(location);
                }
            }
            _ => return false,
        }
        true
    }

    fn install(&mut self, run: Run) {
        if let Some(listed) = self.runs.iter_mut().find(|listed| listed.id == run.id)
            && listed.revision <= run.revision
        {
            *listed = run.clone();
        }
        if self.selected == Some(run.id)
            && self
                .run
                .as_ref()
                .is_none_or(|old| old.id != run.id || old.revision <= run.revision)
        {
            if self.run.as_ref() != Some(&run) {
                self.clear_route();
            }
            self.run = Some(run);
        }
    }
}

fn valid_run(run: &Run) -> bool {
    use super::domain::cards::{COPIES, DECK_MAX, DECK_MIN, MAX_CATALOG};
    let deck_count: u16 = run
        .cards
        .deck
        .iter()
        .take(usize::from(DECK_MAX) + 1)
        .map(|stack| u16::from(stack.copies))
        .sum();
    uuid7(run.id)
        && uuid7(run.cohort)
        && run.economy.validate().is_ok()
        && run.cards.owned.len() <= MAX_CATALOG
        && run
            .cards
            .owned
            .iter()
            .all(|stack| (1..=COPIES).contains(&stack.copies))
        && run.cards.deck.len() <= usize::from(DECK_MAX)
        && run.cards.deck.iter().all(|stack| stack.copies > 0)
        && if let Some(formation) = &run.cards.formation {
            run.shop.generator == super::domain::v2::generation::RUN
                && deck_count == 39
                && formation.sideboard.len() <= 5
                && formation
                    .sideboard
                    .iter()
                    .map(|stack| u16::from(stack.copies))
                    .sum::<u16>()
                    <= 5
                && formation.sideboard.iter().all(|stack| stack.copies > 0)
                && run.cards.copies(formation.commander) == 1
        } else {
            run.shop.generator != super::domain::v2::generation::RUN
                && (DECK_MIN..=DECK_MAX).contains(&deck_count)
        }
        && run.history.len() <= usize::from(super::domain::v2::MAX_STAGES)
        && run.history.iter().all(|duel| {
            duel.bonus_gold <= 42
                && (run.shop.generator == super::domain::v2::generation::RUN
                    || duel.bonus_gold == 0)
        })
        && run.shop.offers.len() == super::domain::v2::shop::OFFERS
        && valid_reward(run)
        && (run.phase != Phase::Playing
            || run.history.len() < usize::from(super::domain::v2::MAX_STAGES))
}

fn valid_reward(run: &Run) -> bool {
    let Some(reward) = &run.reward else {
        return run.phase != Phase::Reward;
    };
    let Some(last) = run.history.last() else {
        return false;
    };
    let allowance = if last.outcome == super::domain::run::Outcome::Victory {
        3
    } else {
        1
    };
    run.phase == Phase::Reward
        && run.shop.generator == super::domain::v2::generation::RUN
        && reward.cards.len() <= 5
        && usize::from(reward.take) == allowance.min(reward.cards.len())
        && reward
            .cards
            .iter()
            .enumerate()
            .all(|(index, card)| !reward.cards[..index].contains(card))
}

fn valid_cohort(cohort: &api::Cohort) -> bool {
    uuid7(cohort.id)
        && cohort.contract.validate(&cohort.catalog).is_ok()
        && cohort.catalog.validate_starters(&cohort.starters).is_ok()
}
