// SPDX-License-Identifier: GPL-3.0-or-later

//! Finding and starting the watermark daemon.

use activate_linux_config::DAEMON_ID;

pub const DAEMON_BIN: &str = "cosmic-ext-activate-linux-daemon";

/// Whether the daemon holds its D-Bus name on the session bus.
pub async fn is_running() -> bool {
    async fn query() -> zbus::Result<bool> {
        let connection = zbus::Connection::session().await?;
        let proxy = zbus::fdo::DBusProxy::new(&connection).await?;
        let name = zbus::names::BusName::try_from(DAEMON_ID)?;
        Ok(proxy.name_has_owner(name).await?)
    }
    query().await.unwrap_or_else(|why| {
        tracing::debug!(%why, "cannot query the daemon's D-Bus name");
        false
    })
}

/// Spawn the daemon detached from this process. Returns whether it was spawned.
pub async fn start() -> bool {
    cosmic::process::spawn(std::process::Command::new(DAEMON_BIN))
        .await
        .is_some()
}
