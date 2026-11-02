use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::{ffi::OsString, path::Path, process::ExitStatus, time::Duration};
use tokio::{
    process::{Child, Command},
    sync::mpsc,
};

type Error = Box<dyn std::error::Error>;

pub async fn run(shutdown_timeout: Duration) -> Result<(), Error> {
    let root = std::env::current_dir()?;
    let (events_tx, mut events) = mpsc::channel(1);
    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<Event>| match result {
            Ok(event) if requires_restart(&event) => {
                let _ = events_tx.try_send(());
            }
            Err(error) => tracing::error!(%error, "Source watcher failed"),
            _ => {}
        })?;
    watcher.watch(&root, RecursiveMode::Recursive)?;
    let executable = std::env::current_exe()?;
    let args: Vec<OsString> = std::env::args_os()
        .skip(1)
        .filter(|arg| arg != "--watch")
        .collect();
    // Install handlers before spawning so an immediate signal cannot orphan the child.
    let shutdown = super::shutdown_signal();
    tokio::pin!(shutdown);
    let mut child = spawn(&executable, &args)?;
    tracing::info!(path = %root.display(), "Watching PHP sources; changes restart the server process");

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                stop(&mut child, shutdown_timeout).await?;
                return Ok(());
            }
            status = child.wait() => {
                let status = status?;
                if !status.success() {
                    return Err(format!("Watched server exited with {status}").into());
                }
                // A successful max-requests drain is replaced just like a source reload.
                child = spawn(&executable, &args)?;
            }
            Some(()) = events.recv() => {
                loop {
                    tokio::select! {
                        _ = &mut shutdown => {
                            stop(&mut child, shutdown_timeout).await?;
                            return Ok(());
                        }
                        Some(()) = events.recv() => {},
                        _ = tokio::time::sleep(Duration::from_millis(300)) => break,
                    }
                }
                tracing::info!("Source changed; draining server before restart");
                stop(&mut child, shutdown_timeout).await?;
                child = spawn(&executable, &args)?;
            }
        }
    }
}

fn spawn(executable: &Path, args: &[OsString]) -> Result<Child, std::io::Error> {
    Command::new(executable)
        .args(args)
        .kill_on_drop(true)
        .spawn()
}

async fn stop(child: &mut Child, timeout: Duration) -> Result<ExitStatus, std::io::Error> {
    if let Some(status) = child.try_wait()? {
        return Ok(status);
    }
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        // Only signal our unreaped direct child; its PID cannot have been reused.
        if unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error);
            }
        }
    }
    #[cfg(not(unix))]
    child.start_kill()?;

    match tokio::time::timeout(timeout + Duration::from_secs(1), child.wait()).await {
        Ok(status) => status,
        Err(_) => {
            child.start_kill()?;
            child.wait().await
        }
    }
}

fn requires_restart(event: &Event) -> bool {
    if !matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) {
        return false;
    }
    event.paths.iter().any(|path| {
        if path.components().any(|part| {
            matches!(
                part.as_os_str().to_str(),
                Some(".git" | "target" | "node_modules" | "vendor" | "storage")
            )
        }) {
            return false;
        }
        path.extension().is_some_and(|extension| extension == "php")
            || path.file_name().is_some_and(|name| {
                name == ".env" || name == "composer.json" || name == "composer.lock"
            })
    })
}
