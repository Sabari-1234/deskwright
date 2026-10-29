use std::sync::Arc;
use std::time::Duration;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::app::AppSelector;
use crate::model::element_query::ElementQuery;
use crate::polling::{PollSchedule, wait_until};
use crate::services::element_search_service::{ElementSearchService, SearchResults};

/// Upper bound on a single wait, so a mistaken call can't hang a client for long.
pub const MAX_WAIT: Duration = Duration::from_secs(30);

/// Waits until elements matching a query appear, instead of sleeping for a guessed time.
pub struct ElementWaitService {
    search: Arc<ElementSearchService>,
    poll_schedule: PollSchedule,
}

impl ElementWaitService {
    /// `poll_schedule` sets the pauses between searches, e.g. a short backoff.
    pub fn new(search: Arc<ElementSearchService>, poll_schedule: impl Into<PollSchedule>) -> Self {
        Self {
            search,
            poll_schedule: poll_schedule.into(),
        }
    }

    /// Searches repeatedly until something matches or `timeout` (capped at [`MAX_WAIT`])
    /// passes. Search failures while waiting (for example, an app still launching) are
    /// retried; the last one is reported if the wait times out.
    pub fn wait_for(
        &self,
        selector: &AppSelector,
        query: &ElementQuery,
        limit: usize,
        timeout: Duration,
    ) -> DeskwrightResult<SearchResults> {
        self.wait_for_matches(selector, query, 1, limit, timeout)
    }

    /// Like [`Self::wait_for`], but waits until at least `minimum_matches` elements match.
    pub fn wait_for_matches(
        &self,
        selector: &AppSelector,
        query: &ElementQuery,
        minimum_matches: usize,
        limit: usize,
        timeout: Duration,
    ) -> DeskwrightResult<SearchResults> {
        let limit = limit.max(minimum_matches);
        let timeout = timeout.min(MAX_WAIT);
        let mut latest = None;
        let mut last_error = None;
        let is_found = wait_until(timeout, self.poll_schedule.clone(), || {
            match self.search.find(selector, query, limit) {
                Ok(results) => {
                    let has_enough = results.matches.len() >= minimum_matches.max(1);
                    latest = Some(results);
                    last_error = None;
                    has_enough
                }
                Err(error) => {
                    last_error = Some(error);
                    false
                }
            }
        });
        if let Some(results) = latest.filter(|_| is_found) {
            return Ok(results);
        }
        Err(DeskwrightError::WaitTimedOut {
            query: query.to_string(),
            timeout_milliseconds: timeout.as_millis(),
            last_problem: last_error
                .map(|error| format!(" (last attempt: {error})"))
                .unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element_registry::ElementRegistry;
    use crate::services::app_resolver::AppResolver;
    use crate::services::snapshot_service::SnapshotService;
    use crate::testing::{FakeDesktop, element};

    fn wait_service(desktop: &Arc<FakeDesktop>) -> ElementWaitService {
        let snapshots = Arc::new(SnapshotService::new(
            AppResolver::new(desktop.clone()),
            desktop.clone(),
            desktop.clone(),
            Arc::new(ElementRegistry::default()),
        ));
        ElementWaitService::new(
            Arc::new(ElementSearchService::new(snapshots)),
            Duration::ZERO,
        )
    }

    #[test]
    fn returns_matches_as_soon_as_they_exist() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Notes", None);
        desktop.add_window(process_id, element("AXButton", Some("Send")));
        let query = ElementQuery::new(Some("button"), None, None).unwrap();
        let selector = AppSelector::NameOrBundleId("Notes".to_owned());

        let results = wait_service(&desktop)
            .wait_for(&selector, &query, 25, Duration::from_secs(5))
            .unwrap();

        assert_eq!(results.matches.len(), 1);
    }

    #[test]
    fn times_out_when_nothing_matches() {
        let desktop = Arc::new(FakeDesktop::new());
        desktop.add_app("Notes", None);
        let query = ElementQuery::new(None, Some("Send"), None).unwrap();
        let selector = AppSelector::NameOrBundleId("Notes".to_owned());

        let error = wait_service(&desktop)
            .wait_for(&selector, &query, 25, Duration::ZERO)
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "no element matching title 'send' appeared within 0 ms"
        );
    }

    #[test]
    fn reports_the_last_failure_when_the_app_never_appears() {
        let desktop = Arc::new(FakeDesktop::new());
        let query = ElementQuery::new(Some("button"), None, None).unwrap();
        let selector = AppSelector::NameOrBundleId("Ghost".to_owned());

        let error = wait_service(&desktop)
            .wait_for(&selector, &query, 25, Duration::ZERO)
            .unwrap_err();

        assert!(
            error
                .to_string()
                .ends_with("(last attempt: no running app matches 'Ghost')")
        );
    }
}
