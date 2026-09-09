use super::*;

#[test]
fn golden_commands_use_api_framing_and_verbatim_adif() {
    assert_eq!(
        Command::update("txtEntryCall", "K7TEST").encode().unwrap(),
        b"<CMD><UPDATE><CONTROL>TXTENTRYCALL</CONTROL><VALUE>K7TEST</VALUE></CMD>\r\n"
    );
    assert_eq!(
        Command::new(CommandKind::List)
            .flag("INCLUDEALL")
            .field("VALUE", "2")
            .encode()
            .unwrap(),
        b"<CMD><LIST><INCLUDEALL><VALUE>2</VALUE></CMD>\r\n"
    );
    let record = "<CALL:6>K7TEST<QSO_DATE:8>20260909<TIME_ON:6>123456<BAND:3>20M<MODE:3>FT8<EOR>";
    assert_eq!(
        String::from_utf8(Command::add_adif(record).encode().unwrap()).unwrap(),
        format!("<CMD><ADDADIFRECORD><VALUE>{record}</VALUE></CMD>\r\n")
    );
    assert!(String::from_utf8(
        Command::update("TXTENTRYCOMMENTS", "A & B")
            .encode()
            .unwrap()
    )
    .unwrap()
    .contains("A & B"));
}
#[test]
fn rejects_frame_injection_invalid_adif_and_missing_fields() {
    for text in ["x</VALUE><RIGTX>", "x\r\n", "café", "x\0"] {
        assert!(Command::update("TXTENTRYCALL", text).encode().is_err());
    }
    for record in [
        "",
        "<CALL:99>X<EOR>",
        "<CALL:1>X<EOR><CALL:1>Y<EOR>",
        "<CALL:1>X</VALUE></CMD>",
        "<COMMENT:10></CMD><CMD><EOR>",
    ] {
        assert!(Command::add_adif(record).encode().is_err(), "{record}");
    }
    assert!(Command::new(CommandKind::Update).encode().is_err());
    assert!(Command::new(CommandKind::Action)
        .field("VALUE", "RIGTX")
        .encode()
        .is_err());
    assert!(Command::new(CommandKind::Read)
        .field("CONTROL", "A")
        .field("control", "B")
        .encode()
        .is_err());
    assert!(Command::new(CommandKind::Program)
        .flag("RIGTX")
        .encode()
        .is_err());
    assert!(Command::new(CommandKind::UpdateAndLog)
        .field("CALL", "K7TEST")
        .field("MODE", "FT8")
        .encode()
        .is_err());
}
#[test]
fn catalog_covers_every_documented_command_and_version_boundaries() {
    let mut names = std::collections::HashSet::new();
    assert_eq!(CommandKind::ALL.len(), 73);
    for kind in CommandKind::ALL {
        assert!(names.insert(kind.spec().name));
        let mut command = Command::new(*kind);
        for key in kind.spec().required_fields {
            command = command.field(*key, "X");
        }
        command = match kind {
            CommandKind::Action => Command::action(Action::Enter),
            CommandKind::AddAdifRecord => Command::add_adif("<CALL:1>X<EOR>"),
            CommandKind::Atno => command.field("DXCC", "291"),
            CommandKind::QsoInProgress | CommandKind::UpdateAndLog => {
                command.field("FREQ", "14.074")
            }
            _ => command,
        };
        assert!(command.encode().is_ok(), "{kind:?}");
    }
    let command = Command::new(CommandKind::GetOtherFieldTitles);
    assert!(!command.supported_by("2.1".parse().unwrap()));
    assert!(command.supported_by("2.2".parse().unwrap()));
    assert!("0.6.2".parse::<Version>().unwrap() < "0.8".parse().unwrap());
    for invalid in ["2", "2.a", "1.2.3.4", "-1.0"] {
        assert!(invalid.parse::<Version>().is_err());
    }
}
#[test]
fn fragmented_coalesced_events_preserve_payload_and_ack_identity() {
    let wire = concat!(
        "<CMD><PROGRAMRESPONSE><PGM>TEST LOGGER</PGM><VER>9.0</VER><APIVER>2.2</APIVER></CMD>\r\n",
        "<CMD><ENTEREVENT><CALL>K7TEST</CALL></CMD>",
        "<CMD><ENTERRESPONSE><VALUE>1</VALUE></CMD>",
        "<CMD><QSORATERESPONSE><20MIN>4</20MIN></CMD>",
        "<CMD><FUTUREEVENT><ROW><CALL>A</CALL></ROW><ROW><CALL>B</CALL></ROW></CMD>"
    )
    .as_bytes();
    for split in 0..=wire.len() {
        let mut decoder = Decoder::new(1024);
        let mut packets = decoder.feed(&wire[..split]).unwrap();
        packets.extend(decoder.feed(&wire[split..]).unwrap());
        assert_eq!(packets.len(), 5);
        assert_eq!(
            packets[0].program_info().unwrap().unwrap().api_version,
            Version(2, 2, 0)
        );
        assert_eq!(packets[1].entered_records().unwrap(), None);
        assert_eq!(packets[2].entered_records().unwrap(), Some(1));
        assert_eq!(packets[3].value("20min"), Some("4"));
        assert_eq!(packets[4].values("CALL"), ["A", "B"]);
        assert_eq!(
            packets[4].values("ROW"),
            ["<CALL>A</CALL>", "<CALL>B</CALL>"]
        );
        assert_eq!(decoder.pending_bytes(), 0);
    }
}
#[test]
fn api_decoder_rejects_malformed_and_limits_memory() {
    for bytes in [
        b"garbage".as_slice(),
        b"<CMD><CMD></CMD>",
        b"<CMD><X>\xff</CMD>",
        b"<CMD></CMD>",
    ] {
        assert!(Decoder::new(128).feed(bytes).is_err());
    }
    let mut decoder = Decoder::new(64);
    assert!(decoder
        .feed(format!("<CMD><X>{}", "A".repeat(10000)).as_bytes())
        .is_err());
    assert_eq!(decoder.pending_bytes(), 0);
    assert_eq!(
        decoder
            .feed(b"<cmd><readresponse><Value>A & B</Value></cmd>\r\n")
            .unwrap()[0]
            .value("value"),
        Some("A & B")
    );
    assert!(Packet::parse("<ENTERRESPONSE><VALUE>NO</VALUE>")
        .unwrap()
        .entered_records()
        .is_err());
}
#[test]
fn entry_sequence_sets_context_and_restores_polling() {
    let entry = Entry {
        call: "K7TEST".into(),
        band: "20".into(),
        mode: "FT8".into(),
        frequency_mhz: None,
        controls: vec![
            ("txtEntryClass".into(), "2A".into()),
            ("txtEntrySection".into(), "WMA".into()),
        ],
    };
    let commands = entry.commands().unwrap();
    assert_eq!(commands[0].kind(), CommandKind::IgnoreRigPolls);
    assert_eq!(commands[1], Command::action(Action::Clear));
    assert_eq!(commands[3], Command::action(Action::CallTab));
    assert_eq!(commands[7].value("VALUE"), Some(""));
    assert_eq!(commands[8], Command::action(Action::Enter));
    assert_eq!(commands[9].value("VALUE"), Some("FALSE"));
    let mut invalid = entry.clone();
    invalid
        .controls
        .push(("TXTENTRYCALL".into(), "OTHER".into()));
    assert!(invalid.commands().is_err());
    for f in ["NaN", "-1", "inf", ""] {
        let mut e = entry.clone();
        e.frequency_mhz = Some(f.into());
        assert!(e.commands().is_err());
    }
}
