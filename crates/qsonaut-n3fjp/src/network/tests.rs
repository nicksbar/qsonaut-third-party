use super::*;

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}
#[test]
fn captured_frames_survive_every_byte_split_and_both_trailers() {
    let mut bytes =
        utf16("<BOR><BAMS><STATION>K7TEST</STATION><BAND>20</BAND><MODE>DIG</MODE><EOR>");
    bytes.extend([3, 4, 7]); // raw trailer described in early capture notes
    bytes.extend(utf16("<BOR><NTWK><TRANSACTION>CLEAR</TRANSACTION><BOR><NTWK><TRANSACTION>LIST</TRANSACTION><EOR>\u{3}\u{4}\u{7}"));
    for split in 0..=bytes.len() {
        let mut decoder = Decoder::new(1024);
        let mut events = decoder.feed(&bytes[..split]).unwrap();
        events.extend(decoder.feed(&bytes[split..]).unwrap());
        assert_eq!(events.len(), 3, "split {split}");
        assert_eq!(
            events[0],
            Message::BandMode {
                station: "K7TEST".into(),
                band: "20".into(),
                mode: "DIG".into()
            }
        );
        assert!(matches!(
            events[1],
            Message::Transaction {
                kind: Transaction::Clear,
                ..
            }
        ));
        assert!(matches!(
            events[2],
            Message::Transaction {
                kind: Transaction::List,
                ..
            }
        ));
    }
}
#[test]
fn network_message_families_and_contest_fields_are_preserved() {
    let cases = [
        (
            "<HELLO>TEST LOGGER<HELLO>",
            Message::Hello("TEST LOGGER".into()),
        ),
        ("<NTWK><OPEN>", Message::Open),
        ("<NTWK><CHECK>", Message::Check),
        (
            "<WHO><STATION>K7TEST</STATION><STATION>W7TEST</STATION>",
            Message::Who(vec!["K7TEST".into(), "W7TEST".into()]),
        ),
        (
            "<MESG><TO></TO><FROM>K7TEST</FROM><MSGTXT>A &amp; B 😀</MSGTXT>",
            Message::Chat {
                to: "".into(),
                from: "K7TEST".into(),
                text: "A & B 😀".into(),
            },
        ),
        (
            "<SCLK><YEAR>2026</YEAR><MILLISECOND>123</MILLISECOND>",
            Message::Clock(vec![
                ("YEAR".into(), "2026".into()),
                ("MILLISECOND".into(), "123".into()),
            ]),
        ),
        (
            "<FUTURE><COUNT>9</COUNT>",
            Message::Unknown("<FUTURE><COUNT>9</COUNT>".into()),
        ),
    ];
    for (body, expected) in cases {
        assert_eq!(Message::parse(body).unwrap(), expected);
        assert_eq!(
            Decoder::new(4096)
                .feed(&encode(&expected).unwrap())
                .unwrap(),
            [expected]
        );
    }
    for kind in ["ADD", "UPDATE", "DELETE", "CHECK", "", "EXTENSION"] {
        let body = format!("<NTWK><FROM>HOST</FROM><TRANSACTION>{kind}</TRANSACTION><XMLDATA><FLDCALL>K7TEST</FLDCALL><FLDPRIMARYKEY>9</FLDPRIMARYKEY><FLDCLASS>2A</FLDCLASS><CUSTOM>x</CUSTOM></XMLDATA>");
        let message = Message::parse(&body).unwrap();
        assert!(
            matches!(&message,Message::Transaction {kind:k,fields,..} if k.as_str()==kind && fields.len()==4)
        );
        assert_eq!(
            Decoder::new(4096).feed(&encode(&message).unwrap()).unwrap(),
            [message]
        );
    }
}
#[test]
fn exact_network_encoding_and_invalid_inputs() {
    assert_eq!(
        encode(&Message::Check).unwrap(),
        utf16("<BOR><NTWK><CHECK></CHECK></NTWK><EOR>\u{3}\u{4}\u{7}")
    );
    assert!(Message::parse("<BAMS><STATION>X</STATION>").is_err());
    assert!(Message::parse("<MESG><FROM>X</FROM><MSGTXT>&unknown;</MSGTXT>").is_err());
    let mut invalid = utf16("<BOR>");
    invalid.extend([0, 0xd8]);
    invalid.extend(utf16("<EOR>"));
    assert!(Decoder::new(1024).feed(&invalid).is_err());
    let mut odd = utf16("<BOR>");
    odd.push(1);
    odd.extend(utf16("<EOR>"));
    assert!(Decoder::new(1024).feed(&odd).is_err());
    let mut decoder = Decoder::new(64);
    assert!(decoder.feed(&vec![1; 1000]).is_err());
    assert_eq!(decoder.pending_bytes(), 0);
    assert!(encode(&Message::Unknown("<BOR>INJECT".into())).is_err());
    assert!(encode(&Message::Chat {
        to: "".into(),
        from: "A".into(),
        text: "bad\0".into()
    })
    .is_err());
}
