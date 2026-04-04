use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

const GH_TIMEOUT_ENV: &str = "GH_VERIFY_GH_TIMEOUT_SECS";
const GH_TIMEOUT_DEFAULT_SECS: u64 = 30;
const POLL_INTERVAL: Duration = Duration::from_millis(50);

pub fn run_gh(args: &[&str]) -> Result<Output> {
    run_command_with_timeout("gh", args, gh_timeout())
}

fn gh_timeout() -> Duration {
    let timeout_secs = std::env::var(GH_TIMEOUT_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .unwrap_or(GH_TIMEOUT_DEFAULT_SECS);
    Duration::from_secs(timeout_secs)
}

fn run_command_with_timeout(command: &str, args: &[&str], timeout: Duration) -> Result<Output> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn `{command}`"))?;

    let stdout = child
        .stdout
        .take()
        .context("failed to capture child stdout")?;
    let stderr = child
        .stderr
        .take()
        .context("failed to capture child stderr")?;
    let stdout_reader = spawn_reader(stdout);
    let stderr_reader = spawn_reader(stderr);

    let started_at = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .with_context(|| format!("failed while waiting for `{command}`"))?
        {
            break status;
        }
        if started_at.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = join_reader(stdout_reader, "stdout");
            let stderr = join_reader(stderr_reader, "stderr").unwrap_or_default();
            let stderr_preview = String::from_utf8_lossy(&stderr).trim().to_string();
            if stderr_preview.is_empty() {
                bail!(
                    "`{} {}` timed out after {}s (set {} to override)",
                    command,
                    args.join(" "),
                    timeout.as_secs(),
                    GH_TIMEOUT_ENV
                );
            }
            bail!(
                "`{} {}` timed out after {}s (set {} to override); stderr: {}",
                command,
                args.join(" "),
                timeout.as_secs(),
                GH_TIMEOUT_ENV,
                stderr_preview
            );
        }
        thread::sleep(POLL_INTERVAL);
    };

    let stdout = join_reader(stdout_reader, "stdout")?;
    let stderr = join_reader(stderr_reader, "stderr")?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn spawn_reader<R>(reader: R) -> thread::JoinHandle<std::io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buf = Vec::new();
        let mut reader = reader;
        reader.read_to_end(&mut buf)?;
        Ok(buf)
    })
}

fn join_reader(
    reader: thread::JoinHandle<std::io::Result<Vec<u8>>>,
    stream_name: &str,
) -> Result<Vec<u8>> {
    match reader.join() {
        Ok(Ok(buf)) => Ok(buf),
        Ok(Err(e)) => Err(anyhow::anyhow!("failed to read child {stream_name}: {e}")),
        Err(_) => Err(anyhow::anyhow!(
            "reader thread panicked while reading child {stream_name}"
        )),
    }
}
