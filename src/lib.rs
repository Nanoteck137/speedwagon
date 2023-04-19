use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};
use enum_primitive_derive::Primitive;
use num_traits::FromPrimitive;

pub const PACKET_START: u8 = 0x4e;

#[derive(Debug)]
pub enum Error {
    ResponseError(ResponseErrorCode),
    PacketNotReponse(PacketType),
    InvalidResponseErrorCode(u8),
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
}
