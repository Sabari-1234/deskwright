//! Starts the real binary and talks MCP to it over stdio. Uses only calls that need no macOS
//! permissions: the handshake, listing tools and apps, and a plain AppleScript expression.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(15);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
const SHUTDOWN_POLL_INTERVAL: Duration = Duration::from_millis(50);
const PROTOCOL_VERSION: &str = "2025-06-18";

/// A running server with an initialized MCP session.
struct ServerProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    responses: mpsc::Receiver<Value>,
    next_request_id: u64,
}

impl ServerProcess {
    fn start(arguments: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_deskwright"))
            .args(arguments)
            .env("RUST_LOG", "warn")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the server binary starts");
        let stdin = child.stdin.take();
        let responses = forward_responses(child.stdout.take().expect("stdout is piped"));
        let mut server = Self {
            child,
            stdin,
            responses,
            next_request_id: 1,
        };
        server.initialize();
        server
    }

    fn initialize(&mut self) {
        let initialize = self.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "smoke-test", "version": "1.0.0" }
            }),
        );
        assert_eq!(initialize["result"]["serverInfo"]["name"], "deskwright");
        self.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_request_id;
        self.next_request_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        self.responses
            .recv_timeout(RESPONSE_TIMEOUT)
            .unwrap_or_else(|_| panic!("no response to {method}"))
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
    }

    fn tool_names(&mut self) -> Vec<String> {
        let listed = self.request("tools/list", json!({}));
        listed["result"]["tools"]
            .as_array()
            .expect("tools is an array")
            .iter()
            .filter_map(|tool| tool["name"].as_str().map(str::to_owned))
            .collect()
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().expect("stdin is open");
        writeln!(stdin, "{message}").expect("write to server");
        stdin.flush().expect("flush to server");
    }

    /// Closes stdin so the server shuts down on its own; kills it only if it hangs.
    fn shut_down(mut self) {
        drop(self.stdin.take());
        let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
        while Instant::now() < deadline {
            if let Ok(Some(status)) = self.child.try_wait() {
                assert!(status.success(), "server exited with {status}");
                return;
            }
            thread::sleep(SHUTDOWN_POLL_INTERVAL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        panic!("server did not exit after stdin closed");
    }
}

/// Forwards each JSON-RPC response line (messages with an `id`) to a channel.
fn forward_responses(stdout: impl std::io::Read + Send + 'static) -> mpsc::Receiver<Value> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if message.get("id").is_some() && sender.send(message).is_err() {
                break;
            }
        }
    });
    receiver
}

fn first_text(response: &Value) -> &str {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
}

#[test]
fn lists_every_tool_except_script_by_default() {
    let mut server = ServerProcess::start(&[]);

    let tool_names = server.tool_names();
    server.shut_down();

    assert_eq!(tool_names.len(), 23);
    assert!(tool_names.iter().any(|name| name == "desk_snapshot"));
    assert!(
        !tool_names.iter().any(|name| name == "desk_script"),
        "script tool must be opt-in"
    );
}

#[test]
fn answers_tool_calls_and_rejects_unknown_tools() {
    let mut server = ServerProcess::start(&[]);

    let apps = server.call_tool("desk_list_apps", json!({}));
    let unknown = server.call_tool("desk_teleport", json!({}));
    server.shut_down();

    assert_eq!(apps["result"]["isError"], false);
    assert!(first_text(&apps).starts_with('['));
    assert!(
        unknown["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("unknown tool 'desk_teleport'")
    );
}

#[test]
fn runs_scripts_only_when_enabled() {
    let mut server = ServerProcess::start(&["--enable-script", "--script-timeout-seconds", "10"]);

    let has_script_tool = server.tool_names().iter().any(|name| name == "desk_script");
    let result = server.call_tool("desk_script", json!({ "code": "return 1 + 1" }));
    server.shut_down();

    assert!(has_script_tool);
    assert_eq!(first_text(&result), "2");
}
