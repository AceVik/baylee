use super::{Account, Endpoint, Error, Fence, Id, SeatRequest, SeatResponse};
use crate::{
    ranked::replica::HostConsentOffer,
    terms::{TermsDocument, TermsSubject},
};

/// The operator named by Central and the exact, digest-checked text to show.
#[derive(Clone, PartialEq, Eq)]
pub struct RankedHostDocument {
    /// Current host identity, function and the account's standing.
    pub offer: HostConsentOffer,
    /// Full verified terms, including translations, if this operator publishes any.
    pub document: Option<TermsDocument>,
}

/// A participant's view of all operators who may receive match data. Bound to
/// the Central account, match, generation and each exact document version.
#[derive(Clone)]
pub struct RankedHostDocuments {
    /// Assigned hosting generation.
    pub fence: Fence,
    /// All named operators whom the UI must display.
    pub hosts: Vec<RankedHostDocument>,
    owner: Id,
    endpoint: Endpoint,
}

impl Account {
    /// Fetch and verify participant-specific operator offers directly from Central.
    ///
    /// # Errors
    /// Refuses a changed assignment, mismatched document or rejected session.
    pub fn ranked_host_documents(&self, match_id: Id) -> Result<RankedHostDocuments, Error> {
        let SeatResponse::Hosts { fence, hosts } = self.replica(SeatRequest::Hosts { match_id })?
        else {
            return Err(Error::Rejected);
        };
        let mut documents = Vec::with_capacity(hosts.len());
        for offer in hosts {
            let document = central_client::gateway_terms(&self.http, &offer.host.origin)?;
            if document.as_ref().map(|d| &d.version) != offer.host.terms.as_ref() {
                return Err(Error::Conflict);
            }
            documents.push(RankedHostDocument { offer, document });
        }
        Ok(RankedHostDocuments {
            fence,
            hosts: documents,
            owner: self.session.identity().account,
            endpoint: self.http.endpoint().clone(),
        })
    }
}

impl RankedHostDocuments {
    fn current(&self, account: &Account) -> Result<Vec<HostConsentOffer>, Error> {
        if self.owner != account.session.identity().account
            || self.endpoint != *account.http.endpoint()
        {
            return Err(Error::Conflict);
        }
        let SeatResponse::Hosts { fence, hosts } = account.replica(SeatRequest::Hosts {
            match_id: self.fence.match_id,
        })?
        else {
            return Err(Error::Rejected);
        };
        if fence != self.fence || !shown(&hosts, &self.hosts) {
            return Err(Error::Conflict);
        }
        Ok(hosts)
    }

    /// Accept the displayed documents and authorize these named operators to
    /// host or verify this match, following an explicit player decision.
    ///
    /// # Errors
    /// Refuses a stale generation or any change to the named operators or terms.
    pub fn accept(&self, account: &Account) -> Result<(), Error> {
        let hosts = self.current(account)?;
        for offer in &hosts {
            if offer.needs_gateway_terms {
                let version = offer.host.terms.as_ref().ok_or(Error::Conflict)?;
                account.session.accept_terms(
                    &account.http,
                    TermsSubject::Gateway {
                        origin: offer.host.origin.clone(),
                    },
                    version,
                )?;
            }
        }
        account.replica(SeatRequest::ApproveHosts {
            fence: self.fence,
            hosts: hosts.into_iter().map(|o| o.host).collect(),
        })?;
        Ok(())
    }

    /// Decline the proposed operators and release the match without a score.
    ///
    /// # Errors
    /// Refuses a replaced or already approved operator or a superseded match.
    pub fn decline(&self, account: &Account) -> Result<(), Error> {
        let current = self.current(account)?;
        let host = current
            .into_iter()
            .find(|o| !o.approved || o.needs_gateway_terms)
            .ok_or(Error::Conflict)?
            .host;
        account.replica(SeatRequest::DeclineHost {
            fence: self.fence,
            host,
        })?;
        Ok(())
    }
}

fn shown(assigned: &[HostConsentOffer], displayed: &[RankedHostDocument]) -> bool {
    assigned.len() == displayed.len()
        && assigned.iter().all(|offer| {
            displayed.iter().any(|d| {
                d.offer.host == offer.host
                    && d.offer.role == offer.role
                    && d.document.as_ref().map(|doc| &doc.version) == offer.host.terms.as_ref()
                    && d.document.as_ref().is_none_or(|doc| {
                        crate::terms::validate(&doc.texts).is_ok()
                            && crate::terms::digest(&doc.texts) == doc.version.digest
                    })
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ranked::replica::{HostApproval, Role};

    #[test]
    fn every_assigned_operator_and_its_actual_text_must_have_been_shown() {
        let offer = |n: u8, role| HostConsentOffer {
            host: HostApproval {
                host_id: [n; 16],
                epoch: 0,
                origin: format!("https://gate{n}.example"),
                terms: None,
            },
            role,
            approved: false,
            needs_gateway_terms: false,
        };
        let a = offer(1, Role::Primary);
        let b = offer(2, Role::Standby);
        let both = [a.clone(), b.clone()];
        let first = RankedHostDocument {
            offer: a,
            document: None,
        };
        let second = RankedHostDocument {
            offer: b,
            document: None,
        };
        assert!(shown(&both, &[first.clone(), second.clone()]));
        assert!(!shown(&both, &[first.clone(), first.clone()]));
        let changed = RankedHostDocument {
            offer: HostConsentOffer {
                host: HostApproval {
                    epoch: 1,
                    ..second.offer.host.clone()
                },
                ..second.offer
            },
            ..second
        };
        assert!(!shown(&both, &[first, changed]));
        let texts = vec![crate::terms::TermsText {
            lang: "de".into(),
            updated: None,
            markdown: "Die gezeigten Regeln.".into(),
        }];
        let version = crate::terms::TermsVersion {
            version: "v1".into(),
            digest: crate::terms::digest(&texts),
        };
        let mut operator = offer(1, Role::Primary);
        operator.host.terms = Some(version.clone());
        let mut displayed = RankedHostDocument {
            offer: operator.clone(),
            document: Some(TermsDocument { version, texts }),
        };
        assert!(shown(
            std::slice::from_ref(&operator),
            std::slice::from_ref(&displayed)
        ));
        displayed.document.as_mut().unwrap().texts[0]
            .markdown
            .push('!');
        assert!(!shown(&[operator], &[displayed]));
    }
}
