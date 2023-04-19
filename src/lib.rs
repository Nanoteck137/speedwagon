use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use enum_primitive_derive::Primitive;
use num_traits::{FromPrimitive, ToPrimitive};

pub const PACKET_START: u8 = 0x4e;

#[derive(Debug)]
pub enum Error {
    ResponseError(ResponseErrorCode),
    PacketNotReponse(PacketType),
    InvalidResponseErrorCode(u8),

    IdentifyErrorReadingVersion(std::io::Error),
    IdentifyErrorReadingNumCmds(std::io::Error),
    IdentifyErrorReadingNameLength(std::io::Error),
    IdentifyErrorReadingName(std::io::Error),
    IdentifyFailedToConvertName(std::string::FromUtf8Error),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Copy, Clone, Primitive, PartialEq, Debug)]
#[repr(u8)]
pub enum PacketType {
    Identify = 0x00,
    Status = 0x01,
    Command = 0x02,
    Ping = 0x03,
    Update = 0x04,
    Response = 0x05,
}

#[derive(Copy, Clone, Primitive, PartialEq, Debug)]
#[repr(u8)]
pub enum ResponseErrorCode {
    Success = 0x00,
    Unknown = 0x01,
    InvalidPacketType = 0x02,
    InvalidCommand = 0x03,
    InvalidDevice = 0x04,
    InsufficientFunctionParameters = 0x05,
    InvalidFunction = 0x06,
}

#[derive(Debug)]
pub struct Packet {
    pid: u8,
    typ: PacketType,
    data: Vec<u8>,
    checksum: u16,
}

impl Packet {
    pub fn pid(&self) -> u8 {
        self.pid
    }

    pub fn typ(&self) -> PacketType {
        self.typ
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn checksum(&self) -> u16 {
        self.checksum
    }

    pub fn response(&self) -> Result<&[u8]> {
        if self.typ == PacketType::Response {
            let error_code = ResponseErrorCode::from_u8(self.data[0])
                .ok_or(Error::InvalidResponseErrorCode(self.data[0]))?;

            match error_code {
                ResponseErrorCode::Success => Ok(&self.data[1..]),
                _ => Err(Error::ResponseError(error_code)),
            }
        } else {
            Err(Error::PacketNotReponse(self.typ))
        }
    }

    pub fn unpack<R>(reader: &mut R) -> Packet
    where
        R: Read,
    {
        let pid = reader.read_u8().unwrap();

        let typ = reader.read_u8().unwrap();
        let typ = PacketType::from_u8(typ).unwrap();

        let data_len = reader.read_u8().unwrap();

        let mut data = vec![0; data_len as usize];
        reader.read_exact(&mut data).unwrap();

        let checksum = reader.read_u16::<LittleEndian>().unwrap();

        return Self {
            pid,
            typ,
            data,
            checksum,
        };
    }

    pub fn pack<W>(writer: &mut W, pid: u8, typ: PacketType, data: &[u8])
    where
        W: Write,
    {
        writer.write_u8(PACKET_START).unwrap();
        writer.write_u8(pid).unwrap(); // PID
        writer.write_u8(typ.to_u8().unwrap()).unwrap();

        // TODO(patrik): Check data.len()
        writer.write_u8(data.len() as u8).unwrap();
        writer.write(data).unwrap();

        writer.write_u16::<LittleEndian>(0).unwrap();
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Version(u16);

impl Version {
    pub fn major(&self) -> u8 {
        ((self.0 >> 10) & 0x3f) as u8
    }

    pub fn minor(&self) -> u8 {
        ((self.0 >> 4) & 0x3f) as u8
    }

    pub fn patch(&self) -> u8 {
        ((self.0) & 0xf) as u8
    }
}

impl std::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Identify {
    pub name: String,
    pub version: Version,
    pub num_cmds: usize,
}

impl Identify {
    pub fn unpack<R>(reader: &mut R) -> Result<Self>
    where
        R: Read,
    {
        let version = reader
            .read_u16::<LittleEndian>()
            .map_err(Error::IdentifyErrorReadingName)?;
        let num_cmds = reader
            .read_u8()
            .map_err(Error::IdentifyErrorReadingNumCmds)?;
        let num_cmds = num_cmds as usize;
        let name_len = reader
            .read_u8()
            .map_err(Error::IdentifyErrorReadingNameLength)?;
        let name_len = name_len as usize;

        let mut buf = vec![0; name_len];
        reader
            .read_exact(&mut buf)
            .map_err(Error::IdentifyErrorReadingName)?;
        let name = String::from_utf8(buf)
            .map_err(Error::IdentifyFailedToConvertName)?;

        Ok(Self {
            name,
            version: Version(version),
            num_cmds,
        })
    }
}
