# qsonaut-n3fjp

Consumer-independent N3FJP protocols, with no modem dependencies. The default
connection settings are disabled. `api` implements the published API 2.2 command
catalog; `network` implements the separate station-network messages documented
by YAHAML's captures. Select the correct protocol and server port explicitly.

```rust,no_run
use qsonaut_n3fjp::{api::{Client, Command, CommandKind}, Config};
use std::{io, time::{Duration, Instant}};

fn discover() -> io::Result<()> {
    let settings = Config { enabled: true, ..Config::default() };
    let Some(mut client) = Client::connect(&settings)? else { return Ok(()); };
    client.send(&Command::new(CommandKind::Program))?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        for packet in client.poll()? {
            // Deliver every packet to your event handler, including unsolicited events.
            println!("{}: {}", packet.id(), packet.body());
        }
        if client.program().is_some() { break; }
    }
    client.disconnect()
}
```

Run synchronous clients on a consumer-owned worker. Persist contacts locally
before forwarding. `send` confirms a socket write only. Inspect `ENTERRESPONSE`
for the logger's reported entry count; a later event, socket EOF, or timeout
must not trigger automatic replay of a possibly saved QSO.

Use `Entry` and `Client::submit_entry` for the scoring-aware entry path after
version discovery. It replaces the current logger entry fields. Use
`Command::add_adif` for direct import when lookup/scoring bypass is appropriate.
`Command::new(CommandKind::...)`, `field`, and `flag` expose all 73 published
commands, including optional parameters and custom contest fields. The enum's
`spec` provides minimum-version and required-field metadata. `Packet` preserves
unknown/nested responses instead of inventing undocumented response schemas.

See [protocol coverage and integration notes](../../docs/N3FJP.md) for examples,
encoding limits, network evidence, transmit authorization, and acceptance gaps.
