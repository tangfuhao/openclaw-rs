use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error};

/// Execute a shell command and return its output.
pub async fn exec_command(program: &str, args: &[&str]) -> anyhow::Result<String> {
    debug!("Executing: {program} {}", args.join(" "));

    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Command failed: {program} - {stderr}");
        anyhow::bail!("Command '{program}' failed with status {}: {stderr}", output.status);
    }
}

/// Kill a process tree by PID.
pub async fn kill_process_tree(pid: u32) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix;
        let _ = Command::new("kill")
            .args(&["-TERM", &format!("-{pid}")])
            .status()
            .await;
    }

    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/T", "/F"])
            .status()
            .await;
    }

    Ok(())
}
