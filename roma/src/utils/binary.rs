use std::io;
use std::io::Read;

use crate::algorithms::checkpoint::CheckpointRunStatus;

// Tag values used when encoding Option<T> in checkpoint binary payloads.
const OPTION_NONE_FLAG: u8 = 0;
const OPTION_SOME_FLAG: u8 = 1;

// Single-byte values used to persist CheckpointRunStatus.
const STATUS_RUNNING_BYTE: u8 = 0;
const STATUS_COMPLETED_BYTE: u8 = 1;
const STATUS_FAILED_BYTE: u8 = 2;
const STATUS_INTERRUPTED_BYTE: u8 = 3;

const ERR_USIZE_TOO_LARGE_TO_SERIALIZE: &str = "usize value too large to serialize into checkpoint";
const ERR_STRING_TOO_LARGE_TO_SERIALIZE: &str = "string too large to serialize into checkpoint";
const ERR_U64_TOO_LARGE_TO_DESERIALIZE_AS_USIZE: &str =
    "u64 value too large to deserialize into usize";
const ERR_INVALID_UTF8_STRING: &str = "invalid UTF-8 string in checkpoint";
const ERR_INVALID_OPTION_FLAG_FOR_STRING_PREFIX: &str = "invalid option flag for string: ";
const ERR_INVALID_OPTION_FLAG_FOR_U64_PREFIX: &str = "invalid option flag for u64: ";
const ERR_INVALID_STATUS_BYTE_PREFIX: &str = "invalid checkpoint status byte: ";

pub(crate) fn push_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

pub(crate) fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[allow(dead_code)]
pub(crate) fn push_f64(out: &mut Vec<u8>, value: f64) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[allow(dead_code)]
pub(crate) fn push_usize(out: &mut Vec<u8>, value: usize) -> io::Result<()> {
    let value = u64::try_from(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            ERR_USIZE_TOO_LARGE_TO_SERIALIZE,
        )
    })?;
    push_u64(out, value);
    Ok(())
}

pub(crate) fn push_string(out: &mut Vec<u8>, value: &str) -> io::Result<()> {
    let bytes = value.as_bytes();
    let len = u32::try_from(bytes.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            ERR_STRING_TOO_LARGE_TO_SERIALIZE,
        )
    })?;
    push_u32(out, len);
    out.extend_from_slice(bytes);
    Ok(())
}

pub(crate) fn push_option_string(out: &mut Vec<u8>, value: &Option<String>) -> io::Result<()> {
    match value {
        Some(text) => {
            push_u8(out, OPTION_SOME_FLAG);
            push_string(out, text)
        }
        None => {
            push_u8(out, OPTION_NONE_FLAG);
            Ok(())
        }
    }
}
#[allow(dead_code)]
pub(crate) fn push_option_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(x) => {
            push_u8(out, OPTION_SOME_FLAG);
            push_u64(out, x);
        }
        None => push_u8(out, OPTION_NONE_FLAG),
    }
}

pub(crate) fn read_u8(input: &mut impl Read) -> io::Result<u8> {
    let mut bytes = [0u8; 1];
    input.read_exact(&mut bytes)?;
    Ok(bytes[0])
}

pub(crate) fn read_u32(input: &mut impl Read) -> io::Result<u32> {
    let mut bytes = [0u8; 4];
    input.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

pub(crate) fn read_u64(input: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0u8; 8];
    input.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[allow(dead_code)]
pub(crate) fn read_f64(input: &mut impl Read) -> io::Result<f64> {
    let mut bytes = [0u8; 8];
    input.read_exact(&mut bytes)?;
    Ok(f64::from_le_bytes(bytes))
}

#[allow(dead_code)]
pub(crate) fn read_usize(input: &mut impl Read) -> io::Result<usize> {
    let value = read_u64(input)?;
    usize::try_from(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            ERR_U64_TOO_LARGE_TO_DESERIALIZE_AS_USIZE,
        )
    })
}

pub(crate) fn read_string(input: &mut impl Read) -> io::Result<String> {
    let len = read_u32(input)? as usize;
    let mut bytes = vec![0u8; len];
    input.read_exact(&mut bytes)?;
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, ERR_INVALID_UTF8_STRING))
}

pub(crate) fn read_option_string(input: &mut impl Read) -> io::Result<Option<String>> {
    match read_u8(input)? {
        OPTION_NONE_FLAG => Ok(None),
        OPTION_SOME_FLAG => Ok(Some(read_string(input)?)),
        flag => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}{}", ERR_INVALID_OPTION_FLAG_FOR_STRING_PREFIX, flag),
        )),
    }
}
#[allow(dead_code)]
pub(crate) fn read_option_u64(input: &mut impl Read) -> io::Result<Option<u64>> {
    match read_u8(input)? {
        OPTION_NONE_FLAG => Ok(None),
        OPTION_SOME_FLAG => Ok(Some(read_u64(input)?)),
        flag => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}{}", ERR_INVALID_OPTION_FLAG_FOR_U64_PREFIX, flag),
        )),
    }
}

pub(crate) fn status_to_byte(status: CheckpointRunStatus) -> u8 {
    match status {
        CheckpointRunStatus::Running => STATUS_RUNNING_BYTE,
        CheckpointRunStatus::Completed => STATUS_COMPLETED_BYTE,
        CheckpointRunStatus::Failed => STATUS_FAILED_BYTE,
        CheckpointRunStatus::Interrupted => STATUS_INTERRUPTED_BYTE,
    }
}

pub(crate) fn byte_to_status(value: u8) -> io::Result<CheckpointRunStatus> {
    match value {
        STATUS_RUNNING_BYTE => Ok(CheckpointRunStatus::Running),
        STATUS_COMPLETED_BYTE => Ok(CheckpointRunStatus::Completed),
        STATUS_FAILED_BYTE => Ok(CheckpointRunStatus::Failed),
        STATUS_INTERRUPTED_BYTE => Ok(CheckpointRunStatus::Interrupted),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}{}", ERR_INVALID_STATUS_BYTE_PREFIX, value),
        )),
    }
}
