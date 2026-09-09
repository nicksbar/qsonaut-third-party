use crate::invalid;
use std::{io, str::FromStr};

/// Numerically comparable API version; 0.0.0 marks commands with no stated minimum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u16, pub u16, pub u16);
impl FromStr for Version {
    type Err = io::Error;
    fn from_str(value: &str) -> io::Result<Self> {
        let parts: Vec<_> = value.trim().split('.').collect();
        if !(2..=3).contains(&parts.len()) {
            return Err(invalid("invalid API version"));
        }
        let mut numbers = [0; 3];
        for (i, part) in parts.iter().enumerate() {
            numbers[i] = part.parse().map_err(|_| invalid("invalid API version"))?;
        }
        Ok(Self(numbers[0], numbers[1], numbers[2]))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CommandSpec {
    pub name: &'static str,
    pub minimum_version: Version,
    pub required_fields: &'static [&'static str],
    /// Direct transmit or arbitrary CAT; requires a live consumer authorization check.
    pub requires_transmit_authorization: bool,
}

macro_rules! catalog {
    ($( $variant:ident => ($name:literal, $version:expr, [$($required:literal),*], $tx:literal) ),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum CommandKind { $( $variant, )* }
        impl CommandKind {
            pub const ALL: &'static [Self] = &[$(Self::$variant,)*];
            pub fn spec(self) -> CommandSpec {
                match self { $( Self::$variant => CommandSpec {
                    name: $name, minimum_version: $version, required_fields: &[$($required),*],
                    requires_transmit_authorization: $tx,
                }, )* }
            }
        }
    }
}
catalog! {
    Update => ("UPDATE", Version(0, 0, 0), ["CONTROL", "VALUE"], false),
    Read => ("READ", Version(0, 0, 0), ["CONTROL"], false),
    Focus => ("FOCUS", Version(0, 0, 0), ["CONTROL"], false),
    Action => ("ACTION", Version(0, 0, 0), ["VALUE"], false),
    SetUpdateState => ("SETUPDATESTATE", Version(0, 0, 0), ["VALUE"], false),
    ReadBmf => ("READBMF", Version(0, 0, 0), [], false),
    ChangeBm => ("CHANGEBM", Version(0, 0, 0), ["BAND", "MODE"], false),
    RigEnabled => ("RIGENABLED", Version(0, 0, 0), [], false),
    ChangeFreq => ("CHANGEFREQ", Version(0, 0, 0), ["VALUE"], false),
    ChangeMode => ("CHANGEMODE", Version(0, 0, 0), ["VALUE"], false),
    DupeCheck => ("DUPECHECK", Version(0, 0, 0), ["CALL", "BAND", "MODE"], false),
    ContestExchange => ("CONTESTEX", Version(1, 3, 0), ["VALUE"], false),
    NextSerialNumber => ("NEXTSERIALNUMBER", Version(0, 0, 0), [], false),
    SetSerialNumber => ("SETSERIALNUMBER", Version(1, 3, 0), ["VALUE"], false),
    Program => ("PROGRAM", Version(0, 0, 0), [], false),
    ApiVersion => ("APIVER", Version(0, 6, 2), [], false),
    FilePath => ("FILEPATH", Version(0, 0, 0), [], false),
    SettingsPath => ("SETTINGSPATH", Version(0, 8, 0), [], false),
    SettingsPathShared => ("SETTINGSPATHSHARED", Version(0, 8, 0), [], false),
    GetUserSettings => ("GETUSERSETTINGS", Version(1, 6, 0), [], false),
    QsoCount => ("QSOCOUNT", Version(0, 0, 0), [], false),
    QsoRate => ("QSORATE", Version(0, 9, 0), [], false),
    SendRigCommand => ("SENDRIGCOMMAND", Version(0, 9, 0), ["VALUE"], true),
    VisibleFields => ("VISIBLEFIELDS", Version(0, 0, 0), [], false),
    AllFields => ("ALLFIELDS", Version(0, 0, 0), [], false),
    AllFieldsWithValues => ("ALLFIELDSWITHVALUES", Version(0, 0, 0), [], false),
    AllFieldsTab => ("ALLFIELDSTAB", Version(1, 1, 0), [], false),
    VisibleFieldsTab => ("VISIBLEFIELDSTAB", Version(1, 1, 0), [], false),
    GetOtherFieldTitles => ("GETOTHERFIELDTITLES", Version(2, 2, 0), [], false),
    MainFormList => ("MAINFORMLIST", Version(0, 0, 0), ["VALUE"], false),
    OperatorInfo => ("OPINFO", Version(0, 6, 2), [], false),
    SetOperatorInfo => ("SETOPINFO", Version(1, 8, 0), [], false),
    GetRigPollingRate => ("GETRIGPOLLINGRATE", Version(1, 9, 0), [], false),
    IgnoreRigPolls => ("IGNORERIGPOLLS", Version(0, 6, 2), ["VALUE"], false),
    UpdateDialogue => ("UPDATEDIALOGUE", Version(0, 6, 2), ["VALUE"], false),
    SpeakText => ("SPEAKTEXT", Version(0, 6, 2), ["VALUE"], false),
    ReadOffsetEnabled => ("READOFFSETENABLED", Version(0, 8, 0), [], false),
    SetOffsetEnabled => ("SETOFFSETENABLED", Version(0, 8, 0), ["VALUE"], false),
    ReadOffset => ("READOFFSET", Version(0, 8, 0), [], false),
    SetOffset => ("SETOFFSET", Version(0, 8, 0), ["VALUE"], false),
    ReadModeDefaultSuppress => ("READMODEDEFAULTSUPPRESS", Version(0, 8, 0), [], false),
    SetModeDefaultSuppress => ("SETMODEDEFAULTSUPPRESS", Version(0, 8, 0), ["VALUE"], false),
    Receive => ("RECEIVE", Version(1, 2, 0), ["IDTAG", "VALUE"], false),
    SetKeyboardState => ("SETKEYBOARDSTATE", Version(0, 6, 2), ["VALUE"], false),
    List => ("LIST", Version(0, 0, 0), [], false),
    Search => ("SEARCH", Version(0, 0, 0), [], false),
    Atno => ("ATNO", Version(0, 7, 0), ["BAND", "MODE"], false),
    GetDynamicResults => ("GETDYNAMICRESULTS", Version(2, 1, 0), [], false),
    CallTabEnterEvents => ("CALLTABENTEREVENTS", Version(0, 8, 0), ["VALUE"], false),
    CountryListLookup => ("COUNTRYLISTLOOKUP", Version(0, 6, 2), ["CALL"], false),
    RelayDxSpots => ("RELAYDXSPOTS", Version(0, 0, 0), ["VALUE"], false),
    FormInfo => ("FORMINFO", Version(0, 6, 2), [], false),
    FormSet => ("FORMSET", Version(0, 6, 2), [], false),
    AddAdifRecord => ("ADDADIFRECORD", Version(2, 0, 0), ["VALUE"], false),
    AddDirect => ("ADDDIRECT", Version(0, 6, 2), [], false),
    CheckLog => ("CHECKLOG", Version(0, 6, 1), [], false),
    OpenLog => ("OPENLOG", Version(0, 0, 0), [], false),
    SendSqlStayOpen => ("SENDSQLSTAYOPEN", Version(0, 0, 0), ["VALUE"], false),
    SqlClose => ("SQLCLOSE", Version(0, 0, 0), [], false),
    CwSend => ("CWSEND", Version(0, 6, 2), ["VALUE"], true),
    CwStop => ("CWSTOP", Version(0, 6, 2), [], false),
    CwSetSpeed => ("CWSETSPEED", Version(0, 6, 2), ["VALUE"], false),
    CwFaster => ("CWFASTER", Version(0, 6, 2), ["VALUE"], false),
    CwSlower => ("CWSLOWER", Version(0, 6, 2), ["VALUE"], false),
    CwComPortKeyDown => ("CWCOMPORTKEYDOWN", Version(0, 8, 0), [], true),
    CwComPortKeyUp => ("CWCOMPORTKEYUP", Version(0, 8, 0), [], false),
    RigTx => ("RIGTX", Version(0, 8, 0), [], true),
    RigRx => ("RIGRX", Version(0, 8, 0), [], false),
    SendRigPoll => ("SENDRIGPOLL", Version(0, 8, 0), ["FREQ", "MODE"], false),
    ReindexWsjt => ("REINDEXWSJT", Version(1, 7, 0), ["MODE", "FREQ"], false),
    QsoInProgress => ("QSOINPROGRESS", Version(1, 7, 0), ["CALL", "MODE"], false),
    GetCallQthInfo => ("GETCALLQTHINFO", Version(1, 7, 0), ["CALL", "BAND", "MODE"], false),
    UpdateAndLog => ("UPDATEANDLOG", Version(1, 7, 0), ["CALL", "MODE"], false),
}
