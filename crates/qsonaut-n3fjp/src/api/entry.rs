use super::{Action, Client, Command, CommandKind};
use crate::invalid;
use std::io;

/// A scored logger entry, using N3FJP's normal ENTER path.
/// Controls carry contest-specific exchanges; discover them via VISIBLEFIELDS.
#[derive(Clone, Debug)]
pub struct Entry {
    pub call: String,
    pub band: String,
    pub mode: String,
    /// MHz, as expected by N3FJP. None explicitly clears stale frequency data.
    pub frequency_mhz: Option<String>,
    pub controls: Vec<(String, String)>,
}
impl Entry {
    /// Validate the complete sequence before any network mutation.
    pub fn commands(&self) -> io::Result<Vec<Command>> {
        if self.call.trim().is_empty() || self.band.trim().is_empty() || self.mode.trim().is_empty()
        {
            return Err(invalid("call, band and mode are required"));
        }
        if let Some(freq) = &self.frequency_mhz {
            let number: f64 = freq.parse().map_err(|_| invalid("frequency must be MHz"))?;
            if !number.is_finite() || number <= 0.0 {
                return Err(invalid("frequency must be positive MHz"));
            }
        }
        let mut commands = vec![
            Command::new(CommandKind::IgnoreRigPolls).field("VALUE", "TRUE"),
            Command::action(Action::Clear),
            Command::update("TXTENTRYCALL", &self.call),
            Command::action(Action::CallTab),
        ];
        let mut seen = std::collections::HashSet::new();
        for (control, value) in &self.controls {
            let control = control.to_ascii_uppercase();
            if !control.starts_with("TXTENTRY")
                || matches!(
                    control.as_str(),
                    "TXTENTRYCALL" | "TXTENTRYBAND" | "TXTENTRYMODE" | "TXTENTRYFREQUENCY"
                )
                || !seen.insert(control.clone())
            {
                return Err(invalid("invalid, duplicate, or reserved entry control"));
            }
            commands.push(Command::update(&control, value));
        }
        commands.extend([
            Command::new(CommandKind::ChangeBm)
                .field("BAND", &self.band)
                .field("MODE", &self.mode),
            Command::update(
                "TXTENTRYFREQUENCY",
                self.frequency_mhz.as_deref().unwrap_or_default(),
            ),
            Command::action(Action::Enter),
            Command::new(CommandKind::IgnoreRigPolls).field("VALUE", "FALSE"),
        ]);
        for command in &commands {
            command.encode()?;
        }
        Ok(commands)
    }
}
impl Client {
    /// Submit through the scoring/lookup-aware ENTER path (API >= 1.4).
    /// This deliberately replaces the logger's current entry fields. The caller
    /// must coordinate with its operator and retain the local contact first.
    /// Poll for ENTERRESPONSE; a successful write alone is not confirmation.
    /// No failed/ambiguous submission is automatically retried.
    pub fn submit_entry(&mut self, entry: &Entry) -> io::Result<()> {
        if self
            .api_version()
            .is_none_or(|v| v < super::Version(1, 4, 0))
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "query PROGRAM/APIVER first; entry helper requires API 1.4",
            ));
        }
        let commands = entry.commands()?;
        // Preflight size limits too, before IGNORERIGPOLLS or CLEAR can be sent.
        for command in &commands {
            self.validate_command(command)?;
        }
        for command in &commands {
            if let Err(error) = self.send(command) {
                let _ =
                    self.send(&Command::new(CommandKind::IgnoreRigPolls).field("VALUE", "FALSE"));
                return Err(error);
            }
        }
        Ok(())
    }
}
