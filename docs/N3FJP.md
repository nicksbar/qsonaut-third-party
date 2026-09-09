# N3FJP protocol adapters

`crates/qsonaut-n3fjp` is a separate workspace crate. It depends only on Serde
for consumer-persisted configuration. It owns wire encoding, bounded stream
framing, and explicit TCP connections; QSONaut retains contact storage, dedupe,
worker scheduling, UI settings, radio ownership, and retry policy.

## Evidence and scope

| Interface | Evidence | Transport | Implementation |
| --- | --- | --- | --- |
| Application API | [N3FJP API 2.2](https://www.n3fjp.com/help/api.html), checked 2026-09-09 | TCP 1100 by default; CMD framing; CRLF writes | `api` |
| Station networking | YAHAML `docs/protocol-summary.md`, `docs/tcp-relay-protocol.md`, `docs/canonical-log-model.md`, `src/relay.ts` at `7af0117b722d95bc8eaf2995a9996b0b6935aa8a` | TCP, commonly 1000/10000; UTF-16LE BOR/EOR framing | `network` |

The published API is separate from the network protocol. API 2.2's documented
addition is custom Other-field title discovery (programs released after
2025-01-01). This is not evidence that N3FJP published its station-network
synchronization protocol. The older YAHAML `n3fjp_captured_protocol.md` predates
the fuller QSO transaction analysis in its protocol summary.

No UDP implementation or consumer migration is included.

## API coverage

The `CommandKind` catalog covers all 73 client commands on the published page.
`Command::field` supplies ordered parameters and `flag("INCLUDEALL")` handles
the unpaired flag used by LIST and SEARCH. Required fields and known minimum
versions are checked; 0.0.0 means the documentation states no minimum. N3FJP
still owns contest-specific validation and application support for a command.

- Field read/update/focus, actions, update subscriptions, field and tab discovery.
- Band/mode/frequency queries and changes, rig state, polling, offsets.
- Duplicate checks, contest exchanges, serial numbers, entity/country queries.
- Program/API version, settings/operator metadata, QSO count/rate, dynamic results.
- List/search, direct ADIF and field-based inserts, SQL operations and refresh.
- WSJT-style QSO progress, lookup, reindex, and UPDATEANDLOG.
- Dialogue/speech, keyboard notifications, DX spots, window settings, WAE QTCs.
- CW and TX command encoding, with explicit consumer authorization for sending
  CWSEND, CWCOMPORTKEYDOWN, RIGTX, and arbitrary SENDRIGCOMMAND.

Packet IDs and original bodies are retained for every response/event, including
unknown IDs, nested LIST/SEARCH records, custom fields, and numeric tags such
as 20MIN. `values` preserves repeated fields. `program_info` and
`entered_records` provide typed access for discovery and entry confirmation.
The client does not assume that the next incoming packet answers the last
command: updates, spots, and other notifications may be interleaved.

The API's tagged text is not well-formed XML. In particular the command ID and
INCLUDEALL flag have no closing tags; ADDADIFRECORD embeds literal ADIF inside
VALUE. Ordinary outgoing values are printable ASCII without angle brackets;
ampersands remain literal. There is no documented XML entity decoding or
Unicode encoding guarantee, so unsupported text is rejected, never silently
replaced. Incoming ASCII/UTF-8 is decoded strictly. A verified Windows code-page
or broader Unicode requirement needs additional live interoperability evidence.

## Configuration and connection lifecycle

`Config` derives Serialize/Deserialize, supplies defaults for missing settings,
and rejects unknown setting names. Defaults:

| Setting | Default |
| --- | --- |
| enabled | false |
| host | 127.0.0.1 |
| port | 1100; `Config::station_network()` uses 1000 |
| connect_timeout_ms | 3000 |
| io_timeout_ms | 1000 |
| max_frame_bytes | 1048576 |

Disabled `connect` returns None before validation, DNS, or socket I/O. An enabled
connection validates its host, port, positive timeouts, and 64-byte to 16-MiB
frame bound. TCP connection attempts share a timeout budget across resolved
addresses. Synchronous OS DNS resolution is outside that budget. I/O timeouts
bound each read/write; callers own overall request/session deadlines.

Connect does not send commands automatically. Query PROGRAM or APIVER and poll
to discover the API version, then configure desired subscriptions. Once a
version is known, newer commands are rejected locally. Each ACTION introduces
a minimum 5-ms delay before another command, including commands sent by helpers.

`poll` returns packets in wire order. An empty result means an incomplete frame
or read timeout; partial input is retained. EOF, malformed frames, and write
errors close the connection. A failed write may have reached the server: it
is an ambiguous outcome, not a retry signal. Reconnect explicitly with a new
client and reestablish metadata/subscriptions. Disable an active integration
by disconnecting its client, then persist `enabled = false`.

## Logging from a consumer

1. Save the contact locally and coordinate ownership of N3FJP's entry form.
2. Discover the connected program/API and its visible fields.
3. Build an `api::Entry` with call, band, mode, optional MHz frequency, and
   contest-specific TXTENTRY controls. Date/time, reports, grid, exchange, and
   other values can be supplied as those controls; no contest is hard-coded.
4. `submit_entry` requires a discovered API >= 1.4. It prevalidates all commands,
   suppresses rig polling, clears stale entry fields, sets the call and performs
   CALLTAB, writes exchange fields, sets band/mode/frequency, performs ENTER,
   and restores rig polling. Missing frequency explicitly clears the field.
5. Continue delivering events and examine ENTERRESPONSE's count. It reports the
   logger's entry result, not a disk-fsync guarantee. ENTEREVENT is a separate
   notification. Do not correlate concurrent ENTER operations without your own
   serialization and operator coordination.

The helper tries to restore rig polling on failure. A broken connection can
prevent restoration; the operator may need to reset IGNORERIGPOLLS in N3FJP.
No helper retries an ambiguous QSO or performs automatic database changes in
response to inbound records.

For direct import, `Command::add_adif` validates a single length-delimited ADIF
record and embeds it verbatim. This uses API >= 2.0 and bypasses normal lookup
and contest scoring. Other direct/SQL commands are explicit low-level operations;
the consumer must close SQL sessions and request refresh as specified by N3FJP.

## Transmit review and consumer gate

The API includes commands that can key a transmitter. Normal `Client::send`
rejects the four commands listed above. `send_with_transmit_authorization`
requires the caller to check its current global arm state after command pacing;
that authorization is never stored. `stop_transmit` explicitly attempts CWSTOP,
CWCOMPORTKEYUP, and RIGRX and reports failures. Incoming SEND/KEYDOWN/CHANGEFREQ
packets remain data; the adapter never forwards them to radio hardware.

Before consumer integration, wire the live authorization callback to the global
arm state and wire global disarm to `stop_transmit`, alongside the consumer's
existing hardware release. A dropped TCP link cannot prove remote PTT release.
Fixture tests use loopback peers and do not key physical hardware. This review
covers the adapter boundary; consumer disarm wiring and live release behavior
remain consumer acceptance work.

## Captured station-network coverage

`network::Message` covers HELLO, BAMS, NTWK OPEN/CHECK, WHO rosters, MESG chat,
SCLK data, and transactions ADD/UPDATE/DELETE/CLEAR/LIST/CHECK/empty ACK. Unknown
transaction names and arbitrary ordered XMLDATA fields are retained. Unknown
message bodies are surfaced explicitly. No timestamps are applied to the system
clock and no inbound CLEAR/DELETE is applied to a local log.

`open_session` sends BAMS, OPEN, and WHO. `heartbeat` sends CHECK when the
caller-selected interval elapses (YAHAML observed approximately 30–60 seconds).
Consumers send BAMS updates, chat, clock, and transactions explicitly and decide
how to respond to server queries. No station identity, contest, geography, or
mode conversion is guessed.

The encoder uses the six-byte UTF-16LE trailer (03 00 04 00 07 00) present in
YAHAML's implementation. The decoder also tolerates the raw three-byte trailer
described in its early notes, odd TCP splits, adjacent records, unclosed
containers, and missing EOR before the next BOR. Unknown synchronization
semantics and uncorrelated ACKs are exposed rather than treated as confirmed
persistence. This is coverage of the captured message families, not a claim to
have recovered every undocumented N3FJP network feature.

## Validation and remaining acceptance

Tests cover golden command bytes, all command IDs, version boundaries, ADIF
lengths/injection rejection, every split point in representative streams,
repeated/nested fields, all captured network families, malformed/oversized
frames, persisted-config defaults, real localhost TCP sessions, timeout/partial
reads, disconnects, action pacing, authorization denial and explicit releases,
metadata discovery, and entry submission with interleaved notifications.

The existing workspace CI test and Clippy steps include this crate. Local
fixtures and host loopback are the evidence here. A running N3FJP application,
multiple contest programs, prolonged network sessions, non-ASCII Windows data,
Android ABI builds, and physical TX release remain unvalidated. Pin this repo
revision before consumer integration and preserve those evidence distinctions.
