use std::io::{Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use deskwright_core::model::script::{Script, ScriptLanguage};
use deskwright_core::ports::ScriptRunner;
use deskwright_core::{DeskwrightError, DeskwrightResult};

const OSASCRIPT_PATH: &str = "/usr/bin/osascript";
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// [`ScriptRunner`] that runs AppleScript or JXA with `/usr/bin/osascript`.
///
/// The source goes through stdin rather than `-e`, so it isn't limited by argument length
/// and doesn't show up in other processes' `ps` output.
#[derive(Debug, Default)]
pub struct OsaScriptRunner;

impl OsaScriptRunner {
    pub fn new() -> Self {
        Self
    }
}

impl ScriptRunner for OsaScriptRunner {
    fn run(&self, script: &Script) -> DeskwrightResult<String> {
        let mut child = Command::new(OSASCRIPT_PATH)
            .args(["-l", language_name(script.language())])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                DeskwrightError::operation_failed("start osascript", error.to_string())
            })?;

        let stdout = read_in_background(child.stdout.take());
        let stderr = read_in_background(child.stderr.take());
        send_source(&mut child, script.source())?;

        let Some(status) = wait_with_timeout(&mut child, script.timeout())? else {
            return Err(DeskwrightError::ScriptTimedOut {
                timeout_seconds: script.timeout().as_secs(),
            });
        };
        let output = join_output(stdout);
        if status.success() {
            Ok(output.trim_end().to_owned())
        } else {
            Err(DeskwrightError::ScriptFailed(
                join_output(stderr).trim().to_owned(),
            ))
        }
    }
}

fn language_name(language: ScriptLanguage) -> &'static str {
    match language {
        ScriptLanguage::AppleScript => "AppleScript",
        ScriptLanguage::JavaScript => "JavaScript",
    }
}

fn send_source(child: &mut Child, source: &str) -> DeskwrightResult<()> {
    // Dropping stdin at the end of this block closes it, which tells osascript the source ended.
    let mut stdin = child.stdin.take().ok_or_else(|| {
        DeskwrightError::operation_failed("send script", "osascript has no stdin")
    })?;
    stdin
        .write_all(source.as_bytes())
        .map_err(|error| DeskwrightError::operation_failed("send script", error.to_string()))
}

/// Waits for the child to exit; kills it and returns `None` once `timeout` passes.
fn wait_with_timeout(child: &mut Child, timeout: Duration) -> DeskwrightResult<Option<ExitStatus>> {
    let deadline = Instant::now() + timeout;
    loop {
        let exit_status = child.try_wait().map_err(|error| {
            DeskwrightError::operation_failed("wait for osascript", error.to_string())
        })?;
        if exit_status.is_some() {
            return Ok(exit_status);
        }
        if Instant::now() >= deadline {
            // The child may exit between the check and the kill; either way it is stopped.
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        thread::sleep(EXIT_POLL_INTERVAL);
    }
}

// Pipes are drained on their own threads so a script with large output can't fill the pipe
// buffer and deadlock while we wait for it to exit.
fn read_in_background(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<String> {
    thread::spawn(move || {
        let mut output = String::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_string(&mut output);
        }
        output
    })
}

fn join_output(reader: JoinHandle<String>) -> String {
    reader.join().unwrap_or_default()
}

// These run the real /usr/bin/osascript. Plain expressions need no macOS permissions, and
// each test finishes in well under a second except the deliberate one-second timeout.
#[cfg(test)]
mod tests {
    use super::*;

    const GENEROUS_TIMEOUT: Duration = Duration::from_secs(20);

    fn run(language: ScriptLanguage, source: &str, timeout: Duration) -> DeskwrightResult<String> {
        let script = Script::new(language, source, timeout).unwrap();
        OsaScriptRunner::new().run(&script)
    }

    #[test]
    fn returns_applescript_output() {
        assert_eq!(
            run(
                ScriptLanguage::AppleScript,
                "return 6 * 7",
                GENEROUS_TIMEOUT
            ),
            Ok("42".to_owned())
        );
    }

    #[test]
    fn returns_javascript_output() {
        assert_eq!(
            run(
                ScriptLanguage::JavaScript,
                "[1, 2].join('-')",
                GENEROUS_TIMEOUT
            ),
            Ok("1-2".to_owned())
        );
    }

    #[test]
    fn reports_the_error_text_of_a_failing_script() {
        let error = run(
            ScriptLanguage::AppleScript,
            "error \"boom\"",
            GENEROUS_TIMEOUT,
        )
        .unwrap_err();

        assert!(
            matches!(&error, DeskwrightError::ScriptFailed(message) if message.contains("boom"))
        );
    }

    #[test]
    fn stops_a_script_that_outlives_its_timeout() {
        let error = run(
            ScriptLanguage::AppleScript,
            "delay 10",
            Duration::from_secs(1),
        )
        .unwrap_err();

        assert_eq!(
            error,
            DeskwrightError::ScriptTimedOut { timeout_seconds: 1 }
        );
    }
}
