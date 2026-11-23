//! Read and accept the exact game-specific set of operators through Central.
use super::{Reply, Worker, consent::localized};
use baylee_account::{
    Account, Error, RankedHostDocuments,
    ranked::replica::{Fence, Role, SeatRequest, SeatResponse},
};
use baylee_client_core::{Phrase, i18n::Lang, terms::TermsDoc};
use std::{fmt::Write as _, sync::Arc};

pub(super) fn read(
    account: &Account,
    fence: Fence,
    cached: Option<Arc<RankedHostDocuments>>,
) -> Result<Arc<RankedHostDocuments>, Error> {
    if let Some(cached) = cached.filter(|d| d.fence == fence) {
        let SeatResponse::Hosts {
            fence: latest,
            hosts,
        } = account.replica(SeatRequest::Hosts {
            match_id: fence.match_id,
        })?
        else {
            return Err(Error::Rejected);
        };
        if latest == fence && cached.hosts.iter().map(|h| &h.offer).eq(&hosts) {
            return Ok(cached);
        }
    }
    let documents = account.ranked_host_documents(fence.match_id)?;
    if documents.fence != fence || documents.hosts.is_empty() || documents.hosts.len() > 2 {
        return Err(Error::Conflict);
    }
    Ok(Arc::new(documents))
}

pub(super) fn sheet(documents: &RankedHostDocuments, lang: Lang) -> Option<TermsDoc> {
    describe(documents.fence, &documents.hosts, lang)
}

fn describe(
    fence: Fence,
    hosts: &[baylee_account::RankedHostDocument],
    lang: Lang,
) -> Option<TermsDoc> {
    let mut markdown = String::from(Phrase::RankedHostsPurpose.text(lang));
    if hosts.len() == 1 {
        let _ = write!(markdown, "\n\n{}", Phrase::RankedSingleHost.text(lang));
    }
    for item in hosts {
        let offer = &item.offer;
        let role = match offer.role {
            Role::Primary => Phrase::RankedHostPlayerFacing,
            Role::Standby => Phrase::RankedHostVerifier,
        };
        let _ = write!(markdown, "\n- {} · {}", role.text(lang), offer.host.origin);
    }
    // List every Central-verified destination before any operator-provided
    // text. Long terms from the first operator cannot obscure the second.
    let mut digest = uuid::Uuid::from_bytes(fence.match_id).to_string();
    for item in hosts {
        let offer = &item.offer;
        let _ = write!(
            digest,
            "-{}-{}-",
            uuid::Uuid::from_bytes(offer.host.host_id),
            offer.host.epoch
        );
        if let Some(version) = &offer.host.terms {
            let source = item.document.as_ref().filter(|d| d.version == *version)?;
            let doc = localized(source, lang)?;
            let _ = write!(
                markdown,
                "\n\n## {}\n\n{}\n\n{}",
                Phrase::TermsTitle.fill(lang, &[&offer.host.origin]),
                Phrase::TermsVersion.fill(lang, &[&doc.version]),
                doc.markdown
            );
            for byte in version.digest {
                let _ = write!(digest, "{byte:02x}");
            }
        }
    }
    Some(TermsDoc {
        version: fence.generation.to_string(),
        digest: Some(digest),
        updated: None,
        markdown,
        lang: Some(lang.code().to_owned()),
    })
}

pub(super) fn answer(worker: &mut Worker) -> bool {
    let (Some(approve), Some(documents)) = (worker.host_choice, worker.documents.clone()) else {
        return false;
    };
    if worker.submit(move |account| {
        Reply::Accepted(if approve {
            documents.accept(account)
        } else {
            documents.decline(account)
        })
    }) {
        worker.host_choice = None;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_account::{
        RankedHostDocument,
        ranked::replica::{HostApproval, HostConsentOffer},
        terms::{TermsDocument, TermsText, TermsVersion},
    };

    #[test]
    fn both_verified_destinations_precede_operator_text_and_changed_terms_need_a_new_read() {
        let fence = Fence {
            match_id: [11; 16],
            generation: 4,
        };
        let version = TermsVersion {
            version: "2026.11".into(),
            digest: [9; 32],
        };
        let primary = RankedHostDocument {
            offer: HostConsentOffer {
                host: HostApproval {
                    epoch: 1,
                    host_id: [1; 16],
                    origin: "https://a.invalid".into(),
                    terms: Some(version.clone()),
                },
                role: Role::Primary,
                approved: false,
                needs_gateway_terms: true,
            },
            document: Some(TermsDocument {
                version,
                texts: vec![TermsText {
                    lang: "en".into(),
                    updated: None,
                    markdown: "body from host A".into(),
                }],
            }),
        };
        let verifier = RankedHostDocument {
            offer: HostConsentOffer {
                host: HostApproval {
                    epoch: 2,
                    host_id: [2; 16],
                    origin: "https://b.invalid".into(),
                    terms: None,
                },
                role: Role::Standby,
                approved: false,
                needs_gateway_terms: false,
            },
            document: None,
        };
        let mut hosts = [primary, verifier];
        let sheet = describe(fence, &hosts, Lang::De).unwrap();
        assert!(
            sheet.markdown.find("https://b.invalid").unwrap()
                < sheet.markdown.find("body from host A").unwrap()
        );
        assert!(
            sheet
                .markdown
                .contains(Phrase::RankedHostVerifier.text(Lang::De))
        );
        hosts[0].offer.host.terms.as_mut().unwrap().digest[0] ^= 1;
        assert!(describe(fence, &hosts, Lang::De).is_none());
        hosts[0].document.as_mut().unwrap().version.digest[0] ^= 1;
        assert_ne!(sheet, describe(fence, &hosts, Lang::De).unwrap());
        hosts[0].document = None;
        assert!(describe(fence, &hosts, Lang::De).is_none());
    }
}
