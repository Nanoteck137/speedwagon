use std::io::{Read, Write};

pub use byteorder::ReadBytesExt;
use byteorder::{LittleEndian, WriteBytesExt};
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

    StateSerializeFailed(std::io::Error),
    StateDeserializeFailed(std::io::Error),
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

    pub fn read<R>(reader: &mut R) -> Result<Packet>
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

        Ok(Self {
            pid,
            typ,
            data,
            checksum,
        })
    }

    pub fn write<W>(
        writer: &mut W,
        pid: u8,
        typ: PacketType,
        data: &[u8],
    ) -> Result<()>
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

        Ok(())
    }

    pub fn write_response<W>(
        writer: &mut W,
        pid: u8,
        error_code: ResponseErrorCode,
        data: &[u8],
    ) -> Result<()>
    where
        W: Write,
    {
        writer.write_u8(PACKET_START).unwrap();
        writer.write_u8(pid).unwrap(); // PID
        let typ = PacketType::Response.to_u8().expect("This should not fail");
        writer.write_u8(typ).unwrap();

        // TODO(patrik): Check data.len()
        let len = data.len() + 1;
        writer.write_u8(len as u8).unwrap();
        writer.write_u8(error_code.to_u8().unwrap()).unwrap();
        writer.write(data).unwrap();

        writer.write_u16::<LittleEndian>(0).unwrap();

        Ok(())
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Version(pub u16);

impl Version {
    pub fn new(major: u8, minor: u8, patch: u8) -> Version {
        Self(
            ((major & 0x3f) as u16) << 10 |
                ((minor & 0x3f) as u16) << 4 |
                (patch & 0xf) as u16,
        )
    }

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

#[derive(Clone, Default, Debug)]
pub struct RSNavState {
    pub led_bar: bool,
    pub led_bar_low_mode: bool,
    pub high_beam: bool,
    pub led_bar_active: bool,

    pub reverse_camera: bool,
    pub reverse_lights: bool,
    pub reverse: bool,
    pub reverse_lights_active: bool,
    pub trunk_lights: bool,
}

impl RSNavState {
    pub const fn new() -> Self {
        Self {
            led_bar: false,
            led_bar_low_mode: false,
            high_beam: false,
            led_bar_active: false,

            reverse_camera: false,
            reverse_lights: false,
            reverse: false,
            reverse_lights_active: false,
            trunk_lights: false,
        }
    }

    pub fn serialize<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        let b = (self.led_bar as u8) << 0 |
            (self.led_bar_low_mode as u8) << 1 |
            (self.high_beam as u8) << 2 |
            (self.led_bar_active as u8) << 3;
        writer.write_u8(b).map_err(Error::StateSerializeFailed)?;

        let b = (self.reverse_camera as u8) << 0 |
            (self.reverse_lights as u8) << 1 |
            (self.reverse as u8) << 2 |
            (self.reverse_lights_active as u8) << 3 |
            (self.trunk_lights as u8) << 4;
        writer.write_u8(b).map_err(Error::StateSerializeFailed)?;

        Ok(())
    }

    pub fn deserialize<R>(reader: &mut R) -> Result<Self>
    where
        R: Read,
    {
        let mut res = Self::default();

        let data = reader.read_u8().map_err(Error::StateDeserializeFailed)?;
        res.led_bar = data & (1 << 0) > 0;
        res.led_bar_low_mode = data & (1 << 1) > 0;
        res.high_beam = data & (1 << 2) > 0;
        res.led_bar_active = data & (1 << 3) > 0;

        let data = reader.read_u8().map_err(Error::StateDeserializeFailed)?;
        res.reverse_camera = data & (1 << 0) > 0;
        res.reverse_lights = data & (1 << 1) > 0;
        res.reverse = data & (1 << 2) > 0;
        res.reverse_lights_active = data & (1 << 3) > 0;
        res.trunk_lights = data & (1 << 4) > 0;

        Ok(res)
    }
}
