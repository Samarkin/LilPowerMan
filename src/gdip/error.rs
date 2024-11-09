use std::fmt::{Debug, Display, Formatter};
use windows::Win32::Graphics::GdiPlus::Status;

pub enum Error {
    GenericError,
    InvalidParameter,
    OutOfMemory,
    ObjectBusy,
    InsufficientBuffer,
    NotImplemented,
    Win32Error,
    WrongState,
    Aborted,
    FileNotFound,
    ValueOverflow,
    AccessDenied,
    UnknownImageFormat,
    FontFamilyNotFound,
    FontStyleNotFound,
    NotTrueTypeFont,
    UnsupportedGdiplusVersion,
    GdiplusNotInitialized,
    PropertyNotFound,
    PropertyNotSupported,
    ProfileNotFound,
    UnknownStatusCode(i32),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn check(status: Status) -> Result<()> {
        match status.0 {
            0 => Ok(()),
            1 => Err(Self::GenericError),
            2 => Err(Self::InvalidParameter),
            3 => Err(Self::OutOfMemory),
            4 => Err(Self::ObjectBusy),
            5 => Err(Self::InsufficientBuffer),
            6 => Err(Self::NotImplemented),
            7 => Err(Self::Win32Error),
            8 => Err(Self::WrongState),
            9 => Err(Self::Aborted),
            10 => Err(Self::FileNotFound),
            11 => Err(Self::ValueOverflow),
            12 => Err(Self::AccessDenied),
            13 => Err(Self::UnknownImageFormat),
            14 => Err(Self::FontFamilyNotFound),
            15 => Err(Self::FontStyleNotFound),
            16 => Err(Self::NotTrueTypeFont),
            17 => Err(Self::UnsupportedGdiplusVersion),
            18 => Err(Self::GdiplusNotInitialized),
            19 => Err(Self::PropertyNotFound),
            20 => Err(Self::PropertyNotSupported),
            21 => Err(Self::ProfileNotFound),
            x => Err(Self::UnknownStatusCode(x)),
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::GenericError => write!(f, "Generic Error"),
            Error::InvalidParameter => write!(f, "Invalid parameter"),
            Error::OutOfMemory => write!(f, "Out of memory"),
            Error::ObjectBusy => write!(f, "Object Busy"),
            Error::InsufficientBuffer => write!(f, "Insufficient buffer"),
            Error::NotImplemented => write!(f, "Not implemented"),
            Error::Win32Error => write!(f, "Win32 error"),
            Error::WrongState => write!(f, "Wrong state"),
            Error::Aborted => write!(f, "Aborted"),
            Error::FileNotFound => write!(f, "File not found"),
            Error::ValueOverflow => write!(f, "Value overflow"),
            Error::AccessDenied => write!(f, "Access denied"),
            Error::UnknownImageFormat => write!(f, "Unknown image format"),
            Error::FontFamilyNotFound => write!(f, "FontFamily not found"),
            Error::FontStyleNotFound => write!(f, "FontStyle not found"),
            Error::NotTrueTypeFont => write!(f, "Not true type font"),
            Error::UnsupportedGdiplusVersion => write!(f, "Unsupported GDI+ version"),
            Error::GdiplusNotInitialized => write!(f, "GDI+ not initialized"),
            Error::PropertyNotFound => write!(f, "Property not found"),
            Error::PropertyNotSupported => write!(f, "Property not supported"),
            Error::ProfileNotFound => write!(f, "Profile not found"),
            Error::UnknownStatusCode(x) => write!(f, "Unknown status code {x}"),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnknownStatusCode(_) => write!(f, "Unknown status code"),
            _ => Debug::fmt(&self, f),
        }
    }
}

impl std::error::Error for Error {}
