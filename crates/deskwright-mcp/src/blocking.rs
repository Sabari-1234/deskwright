use deskwright_core::{DeskwrightError, DeskwrightResult};

/// Runs blocking platform work (Accessibility IPC, scripts, screen capture) on tokio's
/// blocking pool so it never stalls the async protocol loop.
pub async fn run_blocking<Output: Send + 'static>(
    work: impl FnOnce() -> DeskwrightResult<Output> + Send + 'static,
) -> DeskwrightResult<Output> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|join_error| {
            DeskwrightError::operation_failed("run the tool", join_error.to_string())
        })?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_the_work_result() {
        assert_eq!(run_blocking(|| Ok(21 * 2)).await, Ok(42));
    }

    #[tokio::test]
    async fn turns_a_panic_into_an_error_instead_of_crashing_the_server() {
        let result: DeskwrightResult<()> = run_blocking(|| panic!("adapter bug")).await;

        assert!(matches!(
            result,
            Err(DeskwrightError::OperationFailed {
                operation: "run the tool",
                ..
            })
        ));
    }
}
