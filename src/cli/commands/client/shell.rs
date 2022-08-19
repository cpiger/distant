use super::{link::RemoteProcessLink, CliError, CliResult};
use anyhow::Context;
use distant_core::{
    data::{Environment, PtySize},
    DistantChannel, DistantChannelExt, RemoteCommand,
};
use log::*;
use std::time::Duration;
use terminal_size::{terminal_size, Height, Width};
use termwiz::{
    caps::Capabilities,
    input::{InputEvent, KeyCodeEncodeModes, KeyboardEncoding},
    terminal::{new_terminal, Terminal},
};

#[derive(Clone)]
pub struct Shell(DistantChannel);

impl Shell {
    pub fn new(channel: DistantChannel) -> Self {
        Self(channel)
    }

    pub async fn spawn(
        mut self,
        cmd: impl Into<Option<String>>,
        mut environment: Environment,
        persist: bool,
    ) -> CliResult {
        // Automatically add TERM=xterm-256color if not specified
        if !environment.contains_key("TERM") {
            environment.insert("TERM".to_string(), "xterm-256color".to_string());
        }

        // Use provided shell, or determine remote operating system to pick a shell
        let cmd = match cmd.into() {
            Some(cmd) => cmd,
            None => {
                let system_info = self
                    .0
                    .system_info()
                    .await
                    .context("Failed to detect remote operating system")?;
                if system_info.family.eq_ignore_ascii_case("windows") {
                    "cmd.exe".to_string()
                } else {
                    "/bin/sh".to_string()
                }
            }
        };

        let mut proc = RemoteCommand::new()
            .persist(persist)
            .environment(environment)
            .pty(
                terminal_size()
                    .map(|(Width(cols), Height(rows))| PtySize::from_rows_and_cols(rows, cols)),
            )
            .spawn(self.0, &cmd)
            .await
            .with_context(|| format!("Failed to spawn {cmd}"))?;

        // Create a new terminal in raw mode
        let mut terminal = new_terminal(
            Capabilities::new_from_env().context("Failed to load terminal capabilities")?,
        )
        .context("Failed to create terminal")?;
        terminal.set_raw_mode().context("Failed to set raw mode")?;

        let mut stdin = proc.stdin.take().unwrap();
        let resizer = proc.clone_resizer();
        tokio::spawn(async move {
            while let Ok(input) = terminal.poll_input(Some(Duration::new(0, 0))) {
                match input {
                    Some(InputEvent::Key(ev)) => {
                        if let Ok(input) = ev.key.encode(
                            ev.modifiers,
                            KeyCodeEncodeModes {
                                encoding: KeyboardEncoding::Xterm,
                                application_cursor_keys: false,
                                newline_mode: false,
                            },
                            /* is_down */ true,
                        ) {
                            if let Err(x) = stdin.write_str(input).await {
                                error!("Failed to write to stdin of remote process: {}", x);
                                break;
                            }
                        }
                    }
                    Some(InputEvent::Resized { cols, rows }) => {
                        if let Err(x) = resizer
                            .resize(PtySize::from_rows_and_cols(rows as u16, cols as u16))
                            .await
                        {
                            error!("Failed to resize remote process: {}", x);
                            break;
                        }
                    }
                    Some(_) => continue,
                    None => tokio::time::sleep(Duration::from_millis(1)).await,
                }
            }
        });

        // Now, map the remote shell's stdout/stderr to our own process,
        // while stdin is handled by the task above
        let link = RemoteProcessLink::from_remote_pipes(
            None,
            proc.stdout.take().unwrap(),
            proc.stderr.take().unwrap(),
        );

        // Continually loop to check for terminal resize changes while the process is still running
        let status = proc.wait().await.context("Failed to wait for process")?;

        // Shut down our link
        link.shutdown().await;

        if !status.success {
            if let Some(code) = status.code {
                return Err(CliError::Exit(code as u8));
            } else {
                return Err(CliError::FAILURE);
            }
        }

        Ok(())
    }
}
