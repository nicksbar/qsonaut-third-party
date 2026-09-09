use qsonaut_n3fjp::{
    api::{self, Action, Command, CommandKind, Entry},
    network, Config,
};
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};

fn listener() -> (TcpListener, Config) {
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let config = Config {
        enabled: true,
        port: server.local_addr().unwrap().port(),
        io_timeout_ms: 100,
        ..Config::default()
    };
    (server, config)
}
fn read_line(reader: &mut BufReader<TcpStream>) -> String {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    line
}
fn peer(server: TcpListener) -> BufReader<TcpStream> {
    let (stream, _) = server.accept().unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    BufReader::new(stream)
}
fn packets(client: &mut api::Client, count: usize) -> Vec<api::Packet> {
    let until = Instant::now() + Duration::from_secs(3);
    let mut out = Vec::new();
    while out.len() < count {
        assert!(Instant::now() < until);
        out.extend(client.poll().unwrap());
    }
    out
}
#[test]
fn disabled_configuration_does_not_resolve_or_connect() {
    let disabled = Config {
        host: "not a host\0".into(),
        port: 0,
        ..Config::default()
    };
    assert!(api::Client::connect(&disabled).unwrap().is_none());
    assert!(network::Client::connect(&disabled).unwrap().is_none());
    assert!(disabled.validate().is_err());
    assert!(Config {
        enabled: true,
        ..disabled
    }
    .validate()
    .is_err());
    assert_eq!(Config::station_network().port, 1000);
    for c in [
        Config {
            io_timeout_ms: 0,
            ..Config::default()
        },
        Config {
            max_frame_bytes: 1,
            ..Config::default()
        },
    ] {
        assert!(c.validate().is_err());
    }
}
#[test]
fn api_session_discovers_version_submits_entry_and_keeps_notifications() {
    let (server, config) = listener();
    let worker = thread::spawn(move || {
        let mut p = peer(server);
        assert_eq!(read_line(&mut p), "<CMD><PROGRAM></CMD>\r\n");
        p.get_mut()
            .write_all(
                b"<CMD><PROGRAMRESPONSE><PGM>TEST</PGM><VER>1</VER><APIVER>2.2</APIVER></CMD>\r\n",
            )
            .unwrap();
        let expected = [
            "IGNORERIGPOLLS",
            "ACTION",
            "UPDATE",
            "ACTION",
            "UPDATE",
            "CHANGEBM",
            "UPDATE",
            "ACTION",
            "IGNORERIGPOLLS",
        ];
        for id in expected {
            assert!(read_line(&mut p).starts_with(&format!("<CMD><{id}>")));
        }
        p.get_mut().write_all(b"<CMD><ENTEREVENT><CALL>K7TEST</CALL></CMD><CMD><ENTERRESPONSE><VALUE>1</VALUE></CMD>").unwrap();
        assert_eq!(read_line(&mut p), "\r\n");
    });
    let mut client = api::Client::connect(&config).unwrap().unwrap();
    let entry = Entry {
        call: "K7TEST".into(),
        band: "20".into(),
        mode: "FT8".into(),
        frequency_mhz: Some("14.074".into()),
        controls: vec![("TXTENTRYRSTR".into(), "-10".into())],
    };
    assert!(client.submit_entry(&entry).is_err()); // requires version discovery first
    client.send(&Command::new(CommandKind::Program)).unwrap();
    packets(&mut client, 1);
    assert_eq!(client.program().unwrap().name, "TEST");
    client.submit_entry(&entry).unwrap();
    let replies = packets(&mut client, 2);
    assert_eq!(replies[0].entered_records().unwrap(), None);
    assert_eq!(replies[1].entered_records().unwrap(), Some(1));
    client.disconnect().unwrap();
    assert!(!client.is_connected());
    worker.join().unwrap();
}
#[test]
fn action_pacing_and_current_transmit_authorization_are_enforced() {
    let (server, config) = listener();
    let worker = thread::spawn(move || {
        let mut p = peer(server);
        assert!(read_line(&mut p).contains("<VALUE>CLEAR</VALUE>"));
        assert_eq!(read_line(&mut p), "<CMD><PROGRAM></CMD>\r\n");
        assert_eq!(read_line(&mut p), "<CMD><RIGTX></CMD>\r\n");
        for id in ["CWSTOP", "CWCOMPORTKEYUP", "RIGRX"] {
            assert_eq!(read_line(&mut p), format!("<CMD><{id}></CMD>\r\n"));
        }
        assert_eq!(read_line(&mut p), "\r\n");
    });
    let mut c = api::Client::connect(&config).unwrap().unwrap();
    let tx = Command::new(CommandKind::RigTx);
    assert_eq!(
        c.send(&tx).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        c.send_with_transmit_authorization(&tx, || false)
            .unwrap_err()
            .kind(),
        io::ErrorKind::PermissionDenied
    );
    let start = Instant::now();
    c.send(&Command::action(Action::Clear)).unwrap();
    c.send(&Command::new(CommandKind::Program)).unwrap();
    assert!(start.elapsed() >= Duration::from_millis(5));
    c.send_with_transmit_authorization(&tx, || true).unwrap();
    c.stop_transmit().unwrap();
    c.disconnect().unwrap();
    worker.join().unwrap();
}
#[test]
fn old_version_rejection_and_eof_never_replay() {
    let (server, config) = listener();
    let worker = thread::spawn(move || {
        let mut p = peer(server);
        p.get_mut()
            .write_all(b"<CMD><APIVERRESPONSE><APIVER>1.9</APIVER></CMD>")
            .unwrap();
        assert_eq!(read_line(&mut p), "<CMD><PROGRAM></CMD>\r\n");
    });
    let mut c = api::Client::connect(&config).unwrap().unwrap();
    packets(&mut c, 1);
    assert_eq!(
        c.send(&Command::add_adif("<CALL:1>X<EOR>"))
            .unwrap_err()
            .kind(),
        io::ErrorKind::Unsupported
    );
    c.send(&Command::new(CommandKind::Program)).unwrap();
    worker.join().unwrap();
    assert_eq!(c.poll().unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    assert!(!c.is_connected());
    assert_eq!(
        c.send(&Command::new(CommandKind::Program))
            .unwrap_err()
            .kind(),
        io::ErrorKind::NotConnected
    );
}
#[test]
fn malformed_api_input_closes_client() {
    let (server, config) = listener();
    let worker = thread::spawn(move || {
        let mut p = peer(server);
        p.get_mut().write_all(b"NOT CMD").unwrap();
    });
    let mut c = api::Client::connect(&config).unwrap().unwrap();
    assert!(c.poll().is_err());
    assert!(!c.is_connected());
    worker.join().unwrap();
}
#[test]
fn network_handshake_and_transactions_use_utf16() {
    let (server, config) = listener();
    let worker = thread::spawn(move || {
        let mut p = peer(server);
        let mut decoder = network::Decoder::new(4096);
        let mut messages = Vec::new();
        while messages.len() < 4 {
            let mut bytes = [0; 128];
            let n = p.read(&mut bytes).unwrap();
            assert_ne!(n, 0);
            messages.extend(decoder.feed(&bytes[..n]).unwrap());
        }
        assert!(
            matches!(&messages[0],network::Message::BandMode {station,..} if station=="K7TEST")
        );
        assert_eq!(messages[1], network::Message::Open);
        assert_eq!(messages[2], network::Message::Who(vec![]));
        assert!(matches!(
            messages[3],
            network::Message::Transaction {
                kind: network::Transaction::Add,
                ..
            }
        ));
        p.get_mut()
            .write_all(
                &network::encode(&network::Message::Transaction {
                    from: "SERVER".into(),
                    kind: network::Transaction::Ack,
                    fields: vec![],
                })
                .unwrap(),
            )
            .unwrap();
        let mut bytes = [0; 1];
        assert_eq!(p.read(&mut bytes).unwrap(), 0);
    });
    let mut c = network::Client::connect(&config).unwrap().unwrap();
    c.open_session("K7TEST", "20", "DIG").unwrap();
    c.send(&network::Message::Transaction {
        from: "TEST".into(),
        kind: network::Transaction::Add,
        fields: vec![("FLDCALL".into(), "W7TEST".into())],
    })
    .unwrap();
    assert!(!c.heartbeat(Duration::from_secs(60)).unwrap());
    assert!(c.heartbeat(Duration::ZERO).is_err());
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        assert!(Instant::now() < until);
        if !c.poll().unwrap().is_empty() {
            break;
        }
    }
    c.disconnect();
    worker.join().unwrap();
}

#[test]
fn partial_frames_survive_idle_timeout() {
    let (server, config) = listener();
    let (release, ready) = std::sync::mpsc::channel();
    let worker = thread::spawn(move || {
        let mut p = peer(server);
        p.get_mut()
            .write_all(b"<CMD><READRESPONSE><VALUE>PART")
            .unwrap();
        ready.recv_timeout(Duration::from_secs(3)).unwrap();
        p.get_mut().write_all(b"IAL</VALUE></CMD>").unwrap();
        assert_eq!(read_line(&mut p), "\r\n");
    });
    let mut c = api::Client::connect(&config).unwrap().unwrap();
    assert!(c.poll().unwrap().is_empty());
    assert!(c.poll().unwrap().is_empty()); // timeout preserves partial frame
    assert!(c.is_connected());
    release.send(()).unwrap();
    assert_eq!(packets(&mut c, 1)[0].value("VALUE"), Some("PARTIAL"));
    c.disconnect().unwrap();
    worker.join().unwrap();
}
