use enum_primitive_derive::Primitive;

#[derive(Debug)]
pub enum Error {
    PacketError(ResponseErrorCode),
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
    InvalidResponse = 0x04,
    InvalidDevice = 0x05,
    InsufficientFunctionParameters = 0x06,
    InvalidFunction = 0x07,
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
}
