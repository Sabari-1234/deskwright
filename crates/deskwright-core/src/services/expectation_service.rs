use std::sync::Arc;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::element::ElementSummary;
use crate::model::element_condition::{ElementCondition, Observation};
use crate::model::element_target::{Locator, MatchPosition};
use crate::polling::{PollSchedule, wait_until};
use crate::services::element_search_service::ElementSearchService;
use crate::services::element_wait_service::MAX_WAIT;
use crate::services::snapshot_service::MAX_SNAPSHOT_ELEMENTS;

/// Retries an assertion about a query's matches until it holds or times out, like
/// Playwright's web-first `expect(locator)` assertions: no fixed sleeps, no flaky one-shot
/// checks.
pub struct ExpectationService {
    search: Arc<ElementSearchService>,
    poll_schedule: PollSchedule,
}

impl ExpectationService {
    /// `poll_schedule` sets the pauses between searches, e.g. a short backoff.
    pub fn new(search: Arc<ElementSearchService>, poll_schedule: impl Into<PollSchedule>) -> Self {
        Self {
            search,
            poll_schedule: poll_schedule.into(),
        }
    }

    /// Checks `condition` on the locator's matches until it holds (returning the selected
    /// element, if any) or the locator's timeout, capped at [`MAX_WAIT`], passes.
    pub fn expect(
        &self,
        subject: &Locator,
        condition: &dyn ElementCondition,
    ) -> DeskwrightResult<Option<ElementSummary>> {
        let timeout = subject.timeout.min(MAX_WAIT);
        let limit = Self::search_limit(subject.position, condition);
        let mut observed = String::new();
        let mut selected = None;
        let is_met = wait_until(timeout, self.poll_schedule.clone(), || {
            match self.search.find(&subject.app, &subject.query, limit) {
                Ok(results) => {
                    let observation = Observation {
                        matches: &results.matches,
                        selected: Self::select(&results.matches, subject.position),
                    };
                    selected = observation.selected.cloned();
                    match condition.check(&observation) {
                        Ok(()) => true,
                        Err(seen) => {
                            observed = seen;
                            false
                        }
                    }
                }
                Err(error) => {
                    observed = format!("the search failed: {error}");
                    false
                }
            }
        });
        if is_met {
            return Ok(selected);
        }
        Err(DeskwrightError::ExpectationFailed {
            subject: subject.query.to_string(),
            expectation: condition.description(),
            timeout_milliseconds: timeout.as_millis(),
            observed,
        })
    }

    fn search_limit(position: MatchPosition, condition: &dyn ElementCondition) -> usize {
        match position {
            _ if condition.needs_every_match() => MAX_SNAPSHOT_ELEMENTS,
            MatchPosition::Last => MAX_SNAPSHOT_ELEMENTS,
            MatchPosition::Nth(nth) => nth + 1,
        }
    }

    fn select(matches: &[ElementSummary], position: MatchPosition) -> Option<&ElementSummary> {
        match position {
            MatchPosition::Nth(nth) => matches.get(nth),
            MatchPosition::Last => matches.last(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::element_registry::ElementRegistry;
    use crate::model::app::AppSelector;
    use crate::model::element_condition::{HasCount, HasText, IsEnabled, IsHidden};
    use crate::model::element_query::ElementQuery;
    use crate::services::app_resolver::AppResolver;
    use crate::services::snapshot_service::SnapshotService;
    use crate::testing::{FakeDesktop, element};

    fn expectations(desktop: &Arc<FakeDesktop>) -> ExpectationService {
        let snapshots = Arc::new(SnapshotService::new(
            AppResolver::new(desktop.clone()),
            desktop.clone(),
            desktop.clone(),
            Arc::new(ElementRegistry::default()),
        ));
        ExpectationService::new(
            Arc::new(ElementSearchService::new(snapshots)),
            Duration::ZERO,
        )
    }

    fn subject(role: &str, position: MatchPosition) -> Locator {
        Locator {
            app: AppSelector::NameOrBundleId("Mail".to_owned()),
            query: ElementQuery::new(Some(role), None, None).unwrap(),
            position,
            timeout: Duration::ZERO,
        }
    }

    fn mail_with_two_fields(desktop: &FakeDesktop) {
        let process_id = desktop.add_app("Mail", None);
        let window = desktop.add_window(process_id, element("AXWindow", Some("New Message")));
        let to = desktop.add_child(window, element("AXTextField", Some("To")));
        desktop.set_value(to, "pat@example.com");
        let subject = desktop.add_child(window, element("AXTextField", Some("Subject")));
        desktop.set_value(subject, "Invoice");
    }

    #[test]
    fn returns_the_selected_element_once_the_condition_holds() {
        let desktop = Arc::new(FakeDesktop::new());
        mail_with_two_fields(&desktop);
        let condition = HasText {
            text: "invoice".to_owned(),
            is_exact: false,
        };

        let selected = expectations(&desktop)
            .expect(&subject("textfield", MatchPosition::Last), &condition)
            .unwrap();

        assert_eq!(
            selected.and_then(|element| element.attributes.title),
            Some("Subject".to_owned())
        );
    }

    #[test]
    fn counts_every_match() {
        let desktop = Arc::new(FakeDesktop::new());
        mail_with_two_fields(&desktop);

        let result = expectations(&desktop)
            .expect(&subject("textfield", MatchPosition::Nth(0)), &HasCount(2));

        assert!(result.is_ok());
    }

    #[test]
    fn reports_what_it_saw_when_the_condition_never_holds() {
        let desktop = Arc::new(FakeDesktop::new());
        mail_with_two_fields(&desktop);

        let error = expectations(&desktop)
            .expect(
                &subject("textfield", MatchPosition::Nth(0)),
                &IsEnabled(false),
            )
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "expected role 'textfield' to be disabled within 0 ms, but e2 is enabled"
        );
    }

    #[test]
    fn treats_a_missing_element_as_hidden() {
        let desktop = Arc::new(FakeDesktop::new());
        mail_with_two_fields(&desktop);

        let selected = expectations(&desktop)
            .expect(&subject("button", MatchPosition::Nth(0)), &IsHidden)
            .unwrap();

        assert!(selected.is_none());
    }

    #[test]
    fn reports_search_failures() {
        let desktop = Arc::new(FakeDesktop::new());

        let error = expectations(&desktop)
            .expect(&subject("button", MatchPosition::Nth(0)), &IsHidden)
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("but the search failed: no running app")
        );
    }
}
