use super::{Failure, Players, Request, Response, Uuid, project, response};
use baylee_tower::v2::package::Seeds;
use baylee_tower_store::{Entropy, Reserve, Start};

impl Players {
    pub(super) async fn dispatch(
        &self,
        account: Uuid,
        request: Request,
        name: Option<&baylee_tower_api::ranking::Name>,
    ) -> Result<Response, Failure> {
        let page = u8::try_from(baylee_tower_api::PAGE_SIZE).map_err(|_| Failure::Unavailable)?;
        Ok(match request {
            Request::Ranking { cohort, lane } => {
                self.store.ranking_identity(account, name).await?;
                Response::Ranking {
                    board: self.store.ranking(account, cohort, lane).await?,
                }
            }
            Request::Listing { revision, listed } => Response::Listing {
                listing: self.store.listing(account, revision, listed, name).await?,
            },
            Request::Cohorts { before } => self.cohorts(account, before).await?,
            Request::Cohort { cohort } => Response::Cohort {
                cohort: Box::new(
                    self.publication(account, cohort, self.store.published(cohort).await?)
                        .await?,
                ),
            },
            Request::Runs { before } => Response::Runs {
                runs: self
                    .store
                    .runs(account, before, page)
                    .await?
                    .into_iter()
                    .map(project)
                    .collect(),
            },
            Request::Run { run } => response(self.store.get(account, run).await?),
            Request::Locate { run } => Response::Location {
                location: self.store.locate(account, run).await?,
            },
            Request::Hosts { run, before } => Response::Hosts {
                run,
                hosts: self.store.hosts(account, run, before, page).await?,
            },
            Request::Start {
                request,
                cohort,
                starter,
                consent,
            } => response(
                self.store
                    .start(
                        account,
                        request,
                        Start {
                            cohort,
                            starter,
                            consent,
                        },
                        seed()?,
                    )
                    .await?,
            ),
            Request::Command {
                request,
                run,
                revision,
                command,
            } => response(
                self.store
                    .command(account, request, run, revision, &command)
                    .await?,
            ),
            Request::Reserve {
                request,
                run,
                revision,
                host,
            } => response(
                self.store
                    .reserve(
                        account,
                        request,
                        Reserve {
                            run,
                            revision,
                            host,
                        },
                        entropy()?,
                        self.record_bytes,
                    )
                    .await?,
            ),
            Request::WithdrawEchoes {} => {
                self.store.withdraw_echoes(account).await?;
                Response::Withdrawn {}
            }
        })
    }

    async fn cohorts(&self, account: Uuid, before: Option<Uuid>) -> Result<Response, Failure> {
        let bound =
            u8::try_from(baylee_tower_api::COHORT_PAGE_SIZE).map_err(|_| Failure::Unavailable)?;
        let saved = self.store.cohorts(before, bound).await?;
        let mut cohorts = Vec::with_capacity(saved.len());
        for (id, publication) in saved {
            cohorts.push(self.publication(account, id, publication).await?);
        }
        Ok(Response::Cohorts {
            consent: baylee_tower_store::CONSENT_REVISION,
            cohorts,
        })
    }

    async fn publication(
        &self,
        account: Uuid,
        id: Uuid,
        saved: baylee_tower_store::Publication,
    ) -> Result<baylee_tower_api::Cohort, Failure> {
        let offered = if saved.cohort.generator == baylee_tower::v2::generation::EXPEDITIONS {
            self.store.starter_choices(account, id, seed()?).await?
        } else {
            let count = u8::try_from(saved.starters.len()).map_err(|_| Failure::Unavailable)?;
            (0..count).collect()
        };
        Ok(baylee_tower_api::Cohort {
            id,
            contract: saved.cohort,
            catalog: saved.catalog,
            starters: saved.starters,
            offered,
        })
    }
}

fn seed<const N: usize>() -> Result<[u8; N], Failure> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(|_| Failure::Unavailable)?;
    Ok(bytes)
}

fn entropy() -> Result<Entropy, Failure> {
    Ok(Entropy {
        opponent: seed()?,
        game: Seeds {
            shuffle: u64::from_le_bytes(seed()?),
            policy: u64::from_le_bytes(seed()?),
        },
    })
}
