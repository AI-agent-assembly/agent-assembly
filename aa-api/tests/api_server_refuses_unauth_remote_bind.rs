//! Regression test for AAASM-6056.
//!
//! `aa-api-server` must refuse to start when bound to a non-loopback address
//! with `AASM_API_AUTH=off` — that combination serves an unauthenticated
//! admin API on the network. Without the check the process binds an ephemeral
//! port on `0.0.0.0` and runs forever, so this drives the binary with
//! `spawn()` + a bounded poll rather than `.output()`, which would hang.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The exact substring the success banner prints — must NOT appear when
/// startup is refused (AAASM-4572 ordering: gate before banner).
const SERVING_BANNER: &str = "serving full /api/v1/* REST surface";

const POLL_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn non_loopback_bind_with_auth_off_is_refused() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aa-api-server"))
        .env("AA_API_ADDR", "0.0.0.0:0")
        .env("AASM_API_AUTH", "off")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn aa-api-server");

    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll child") {
            break status;
        }
        if start.elapsed() > POLL_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "aa-api-server did not exit within {POLL_TIMEOUT:?} — the \
                 non-loopback + AASM_API_AUTH=off refusal is missing or broken, \
                 and the process is instead serving an unauthenticated admin API"
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    assert!(
        !status.success(),
        "expected a non-zero exit for a non-loopback bind with AASM_API_AUTH=off, got {status:?}"
    );

    let mut stderr = String::new();
    use std::io::Read;
    child
        .stderr
        .take()
        .expect("captured stderr")
        .read_to_string(&mut stderr)
        .expect("read stderr");

    assert!(
        !stderr.contains(SERVING_BANNER),
        "the '{SERVING_BANNER}' banner must NOT be printed when startup is refused \
         — it implies the server is up. stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("AASM_API_AUTH"),
        "expected the refusal message to name AASM_API_AUTH. stderr was:\n{stderr}"
    );
}
