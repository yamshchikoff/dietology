// Integration test: browser streaming UI verification.
//
// Starts the mock LLM server, then runs Puppeteer-based browser tests
// that verify streaming messages are correctly displayed in the DOM.
//
// Requires: Node.js (v22+), npm dependencies installed in web/test/
// Install:  cd web/test && npm install
//
// Chromium is auto-downloaded by Puppeteer on first run.

use std::path::PathBuf;
use std::process::{Command, Stdio};

#[test]
fn browser_streaming_tests() {
    // Check Node.js is available
    let node_check = Command::new("node")
        .arg("--version")
        .stdout(Stdio::null())
        .status();

    if node_check.is_err() || !node_check.unwrap().success() {
        eprintln!("SKIP: Node.js not available — required for browser tests");
        return;
    }

    // Project root is the parent of src-tauri/
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("no parent of manifest dir")
        .to_path_buf();

    // Run the Puppeteer test script which handles mock server lifecycle internally
    let status = Command::new("node")
        .arg("web/test/browser_test.mjs")
        .current_dir(&project_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("failed to run browser test script");

    assert!(status.success(), "Browser streaming tests failed");
}
