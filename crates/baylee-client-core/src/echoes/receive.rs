use super::{Error, Phase, Request, Response, Run, State, Work, api, mutation, uuid7};

impl State {
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
            (Request::Cohorts { .. }, Response::Cohorts { consent, cohorts })
                if consent > 0
                    && cohorts.len() <= api::PAGE_SIZE
                    && cohorts.iter().all(|c| uuid7(c.id)) =>
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
    uuid7(run.id)
        && uuid7(run.cohort)
        && run.history.len() <= usize::from(super::domain::v2::MAX_STAGES)
        && run.shop.offers.len() == super::domain::v2::shop::OFFERS
        && (run.phase != Phase::Playing
            || run.history.len() < usize::from(super::domain::v2::MAX_STAGES))
}
