use crate::constants;
use clap::Args;
use serde::{Deserialize, Serialize};

/// Level of access control to the unix socket or windows pipe
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[clap(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AccessControl {
    /// Equates to `0o600` on Unix (read & write for owner)
    Owner,

    /// Equates to `0o660` on Unix (read & write for owner and group)
    Group,

    /// Equates to `0o666` on Unix (read & write for owner, group, and other)
    Anyone,
}

impl AccessControl {
    /// Converts into a Unix file permission octal
    pub fn into_mode(self) -> u32 {
        match self {
            Self::Owner => 0o600,
            Self::Group => 0o660,
            Self::Anyone => 0o666,
        }
    }
}

impl Default for AccessControl {
    /// Defaults to owner-only permissions
    fn default() -> Self {
        Self::Owner
    }
}

/// Represents common networking configuration
#[derive(Args, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkSettings {
    /// Override the path to the Unix socket used by the manager (unix-only)
    #[clap(long)]
    pub unix_socket: Option<std::path::PathBuf>,

    /// Override the name of the local named Windows pipe used by the manager (windows-only)
    #[clap(long)]
    pub windows_pipe: Option<String>,
}

impl NetworkSettings {
    /// Merge these settings with the `other` settings. These settings take priority
    /// over the `other` settings.
    pub fn merge(&mut self, other: Self) {
        self.unix_socket = self.unix_socket.take().or(other.unix_socket);
        self.windows_pipe = self.windows_pipe.take().or(other.windows_pipe);
    }

    /// Returns option containing reference to unix path if configured
    pub fn as_unix_socket_opt(&self) -> Option<&std::path::Path> {
        self.unix_socket.as_deref()
    }

    /// Returns option containing reference to windows pipe name if configured
    pub fn as_windows_pipe_opt(&self) -> Option<&str> {
        self.windows_pipe.as_deref()
    }

    /// Returns a collection of candidate unix socket paths, which will either be
    /// the config-provided unix socket path or the default user and global socket paths
    pub fn to_unix_socket_path_candidates(&self) -> Vec<&std::path::Path> {
        match self.unix_socket.as_deref() {
            Some(path) => vec![path],
            None => vec![
                constants::user::UNIX_SOCKET_PATH.as_path(),
                constants::global::UNIX_SOCKET_PATH.as_path(),
            ],
        }
    }

    /// Returns a collection of candidate windows pipe names, which will either be
    /// the config-provided windows pipe name or the default user and global pipe names
    pub fn to_windows_pipe_name_candidates(&self) -> Vec<&str> {
        match self.windows_pipe.as_deref() {
            Some(name) => vec![name],
            None => vec![
                constants::user::WINDOWS_PIPE_NAME.as_str(),
                constants::global::WINDOWS_PIPE_NAME.as_str(),
            ],
        }
    }
}
