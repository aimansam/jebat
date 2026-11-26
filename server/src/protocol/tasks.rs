use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TaskType {
    Nop = 0,
    Shell = 1,
    SleepChange = 2,
    Exit = 3,
    FileUpload = 4,
    FileDownload = 5,
    CredDump = 6,
    LateralMove = 7,
    Exfil = 8,
    SocksConnect = 9,
    PortForward = 10,
}

impl TaskType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Nop),
            1 => Some(Self::Shell),
            2 => Some(Self::SleepChange),
            3 => Some(Self::Exit),
            4 => Some(Self::FileUpload),
            5 => Some(Self::FileDownload),
            6 => Some(Self::CredDump),
            7 => Some(Self::LateralMove),
            8 => Some(Self::Exfil),
            9 => Some(Self::SocksConnect),
            10 => Some(Self::PortForward),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Nop => "nop",
            Self::Shell => "shell",
            Self::SleepChange => "sleep_change",
            Self::Exit => "exit",
            Self::FileUpload => "file_upload",
            Self::FileDownload => "file_download",
            Self::CredDump => "cred_dump",
            Self::LateralMove => "lateral_move",
            Self::Exfil => "exfil",
            Self::SocksConnect => "socks_connect",
            Self::PortForward => "port_forward",
        }
    }
}
