//! Integration tests for the standalone dashboard lifecycle.

use serde_json::Value;
use std::process::{Child, Command, Output, Stdio};
use tempfile::TempDir;

const BIN: &str = env!("CARGO_BIN_EXE_agent-browser");

struct DashboardCleanup<'a>(&'a TempDir);

impl Drop for DashboardCleanup<'_> {
    fn drop(&mut self) {
        let _ = run_dashboard(self.0, &["dashboard", "stop", "--json"]);
    }
}

struct RunningDashboard(Child);

impl Drop for RunningDashboard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn socket_dir(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join("sockets")
}

fn seed_running_dashboard(tmp: &TempDir, port: u16, allowed_origins: &[&str]) -> RunningDashboard {
    let socket_dir = socket_dir(tmp);
    std::fs::create_dir_all(&socket_dir).unwrap();
    let child = Command::new(BIN)
        .env("AGENT_BROWSER_DASHBOARD", "1")
        .env("AGENT_BROWSER_DASHBOARD_PORT", "0")
        .env("AGENT_BROWSER_SOCKET_DIR", &socket_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to seed a running dashboard process");
    let config = serde_json::json!({
        "port": port,
        "allowed_origins": allowed_origins,
    });
    std::fs::write(
        socket_dir.join("dashboard.config"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    std::fs::write(socket_dir.join("dashboard.pid"), child.id().to_string()).unwrap();
    RunningDashboard(child)
}

fn run_dashboard(tmp: &TempDir, args: &[&str]) -> Output {
    let socket_dir = socket_dir(tmp);
    std::fs::create_dir_all(&socket_dir).unwrap();

    Command::new(BIN)
        .args(args)
        .env("AGENT_BROWSER_SOCKET_DIR", socket_dir)
        .env_remove("AGENT_BROWSER_DASHBOARD_ALLOWED_ORIGINS")
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to invoke agent-browser dashboard")
}

fn json_output(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout was not JSON: {error}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn explicit_dashboard_start_accepts_mcp_style_arguments() {
    let tmp = TempDir::new().unwrap();
    let _cleanup = DashboardCleanup(&tmp);
    let port = 0;
    let port_arg = port.to_string();

    let started = run_dashboard(
        &tmp,
        &[
            "dashboard",
            "start",
            "--port",
            &port_arg,
            "--allowed-origins",
            "https://dashboard.example.com",
            "--json",
        ],
    );
    assert!(
        started.status.success(),
        "dashboard start failed: {}",
        String::from_utf8_lossy(&started.stderr)
    );
    assert_eq!(json_output(&started)["data"]["port"], port);
}

#[test]
fn running_dashboard_rejects_configuration_changes() {
    let tmp = TempDir::new().unwrap();
    let port = 0;
    let _dashboard = seed_running_dashboard(
        &tmp,
        port,
        &[
            "https://dashboard.example.com",
            "https://second.example.com",
        ],
    );
    let port_arg = port.to_string();
    let start_args = [
        "dashboard",
        "--port",
        &port_arg,
        "--allowed-origins",
        "https://dashboard.example.com,https://second.example.com",
        "--json",
    ];

    let repeated = run_dashboard(&tmp, &start_args);
    assert!(repeated.status.success());
    assert_eq!(json_output(&repeated)["data"]["already_running"], true);

    let changed = run_dashboard(
        &tmp,
        &[
            "dashboard",
            "start",
            "--port",
            &port_arg,
            "--allowed-origins",
            "https://different.example.com",
            "--json",
        ],
    );
    assert!(
        !changed.status.success(),
        "changed allowlist unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&changed.stdout),
        String::from_utf8_lossy(&changed.stderr)
    );
    let payload = json_output(&changed);
    assert_eq!(payload["success"], false);
    assert!(payload["error"]
        .as_str()
        .is_some_and(|error| error.contains("dashboard stop")));

    let changed_port = "1";
    let changed = run_dashboard(
        &tmp,
        &[
            "dashboard",
            "start",
            "--port",
            changed_port,
            "--allowed-origins",
            "https://dashboard.example.com,https://second.example.com",
            "--json",
        ],
    );
    assert!(
        !changed.status.success(),
        "changed port unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&changed.stdout),
        String::from_utf8_lossy(&changed.stderr)
    );
    assert!(json_output(&changed)["error"]
        .as_str()
        .is_some_and(|error| error.contains("dashboard stop")));
}
