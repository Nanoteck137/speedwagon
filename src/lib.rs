use std::io::{Read, Write};

pub use byteorder::ReadBytesExt;
use byteorder::{LittleEndian, WriteBytesExt};
use enum_primitive_derive::Primitive;
use num_traits::{FromPrimitive, ToPrimitive};

pub const PACKET_START: u8 = 0x4e;
pub const NUM_STATUS_BYTES: usize = 8;
pub const NUM_CMD_PARAMS: usize = 8;

#[derive(Debug)]
pub enum Error {
    InvalidResponseCode(u8),
    InvalidPacketType,

    PacketSerialize(std::io::Error),
    PacketDeserialize(std::io::Error),

    IdentitySerialize(std::io::Error),
    IdentityDeserialize(std::io::Error),
    IdentityInvalidName(std::string::FromUtf8Error),

    StateSerializeFailed(std::io::Error),
    StateDeserializeFailed(std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Copy, Clone, Primitive, PartialEq, Debug)]
#[repr(u8)]
pub enum ResponseCode {
    Success = 0x00,
    Unknown = 0x01,
    InvalidPacketType = 0x02,
    InvalidCommand = 0x03,
    InsufficientFunctionParameters = 0x05,
}

#[derive(Debug)]
pub enum PacketType {
    Connect {
        send_status: bool,
        status_time: u16,
    },
    Disconnect,
    Error {
        code: ResponseCode,
    },

    Cmd {
        index: u8,
        params: [u8; NUM_CMD_PARAMS],
    },
    Identify,
    Status,

    OnConnect,
    OnCmd,
    OnIdentify(Identity),
    OnStatus([u8; NUM_STATUS_BYTES]),
}

impl PacketType {
    fn to_u8(&self) -> u8 {
        match self {
            PacketType::Connect {
                send_status: _,
                status_time: _,
            } => 0,
            PacketType::Disconnect => 1,
            PacketType::Error { code: _ } => 2,

            PacketType::Cmd {
                index: _,
                params: _,
            } => 3,
            PacketType::Identify => 4,
            PacketType::Status => 5,

            PacketType::OnConnect => 6,
            PacketType::OnCmd => 7,
            PacketType::OnIdentify(_) => 8,
            PacketType::OnStatus(_) => 9,
        }
    }
}

#[derive(Debug)]
pub struct Packet {
    id: u16,
    typ: PacketType,
}

impl Packet {
    pub fn new(id: u16, typ: PacketType) -> Self {
        Self { id, typ }
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn typ(&self) -> &PacketType {
        &self.typ
    }

    pub fn serialize<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        writer
            .write_u16::<LittleEndian>(self.id)
            .map_err(Error::PacketSerialize)?;
        writer
            .write_u8(self.typ.to_u8())
            .map_err(Error::PacketSerialize)?;

        match &self.typ {
            PacketType::Connect {
                send_status,
                status_time,
            } => {
                writer
                    .write_u8(*send_status as u8)
                    .map_err(Error::PacketSerialize)?;
                writer
                    .write_u16::<LittleEndian>(*status_time)
                    .map_err(Error::PacketSerialize)?;
            }
            PacketType::Disconnect => {}

            PacketType::Error { code } => {
                let code = code.to_u8().unwrap();
                writer.write_u8(code).map_err(Error::PacketSerialize)?;
            }

            PacketType::Cmd { index, params } => {
                writer.write_u8(*index).map_err(Error::PacketSerialize)?;
                writer.write(params).map_err(Error::PacketSerialize)?;
            }

            PacketType::Identify => {}
            PacketType::Status => {}
            PacketType::OnConnect => {}
            PacketType::OnCmd => {}

            PacketType::OnIdentify(identity) => identity.serialize(writer)?,
            PacketType::OnStatus(status) => {
                writer.write(status).map_err(Error::PacketSerialize)?;
            }
        }

        Ok(())
    }

    pub fn deserialize<R>(reader: &mut R) -> Result<Self>
    where
        R: Read,
    {
        let id = reader
            .read_u16::<LittleEndian>()
            .map_err(Error::PacketDeserialize)?;
        let typ = reader.read_u8().map_err(Error::PacketDeserialize)?;

        let typ = match typ {
            0 => {
                let send_status =
                    reader.read_u8().map_err(Error::PacketDeserialize)?;
                let send_status = send_status > 0;
                let status_time = reader
                    .read_u16::<LittleEndian>()
                    .map_err(Error::PacketDeserialize)?;

                Ok(PacketType::Connect {
                    send_status,
                    status_time,
                })
            }
            1 => Ok(PacketType::Disconnect),
            2 => {
                let code =
                    reader.read_u8().map_err(Error::PacketDeserialize)?;
                let code = ResponseCode::from_u8(code)
                    .ok_or(Error::InvalidResponseCode(code))?;

                Ok(PacketType::Error { code })
            }

            3 => {
                let index =
                    reader.read_u8().map_err(Error::PacketDeserialize)?;

                let mut params = [0; NUM_CMD_PARAMS];
                reader
                    .read_exact(&mut params)
                    .map_err(Error::PacketDeserialize)?;

                Ok(PacketType::Cmd { index, params })
            }

            4 => Ok(PacketType::Identify),
            5 => Ok(PacketType::Status),
            6 => Ok(PacketType::OnConnect),
            7 => Ok(PacketType::OnCmd),

            8 => {
                let identity = Identity::deserialize(reader)?;
                Ok(PacketType::OnIdentify(identity))
            }

            9 => {
                let mut status = [0; NUM_STATUS_BYTES];
                reader
                    .read_exact(&mut status)
                    .map_err(Error::PacketDeserialize)?;
                Ok(PacketType::OnStatus(status))
            }

            _ => Err(Error::InvalidPacketType),
        };

        let typ = typ?;

        Ok(Packet { id, typ })
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
pub struct Identity {
    pub name: String,
    pub version: Version,
    pub num_cmds: usize,
}

impl Identity {
    pub fn serialize<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        writer
            .write_u16::<LittleEndian>(self.version.0)
            .map_err(Error::IdentitySerialize)?;
        // TODO(patrik): Check num_cmds
        writer
            .write_u8(self.num_cmds as u8)
            .map_err(Error::IdentitySerialize)?;
        // TODO(patrik): Check name len
        writer
            .write_u8(self.name.len() as u8)
            .map_err(Error::IdentitySerialize)?;
        writer
            .write(self.name.as_bytes())
            .map_err(Error::IdentitySerialize)?;

        Ok(())
    }

    pub fn deserialize<R>(reader: &mut R) -> Result<Self>
    where
        R: Read,
    {
        let version = reader
            .read_u16::<LittleEndian>()
            .map_err(Error::IdentityDeserialize)?;
        let num_cmds = reader.read_u8().map_err(Error::IdentityDeserialize)?;
        let num_cmds = num_cmds as usize;
        let name_len = reader.read_u8().map_err(Error::IdentityDeserialize)?;
        let name_len = name_len as usize;

        let mut buf = vec![0; name_len];
        reader
            .read_exact(&mut buf)
            .map_err(Error::IdentityDeserialize)?;
        let name =
            String::from_utf8(buf).map_err(Error::IdentityInvalidName)?;

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

    pub fn set_led_bar_active(&mut self, on: bool) {
        self.led_bar_active = on;

        if self.led_bar_active {
            self.led_bar = self.high_beam;
        } else {
            self.led_bar = false;
        }
    }

    pub fn set_led_bar_low_mode(&mut self, on: bool) {
        self.led_bar_low_mode = on;
    }

    pub fn force_led_bar(&mut self, on: bool) {
        self.led_bar = on;
    }

    pub fn set_trunk_lights(&mut self, on: bool) {
        self.trunk_lights = on;
    }

    pub fn set_reverse_lights_active(&mut self, on: bool) {
        self.reverse_lights_active = on;

        if self.reverse_lights_active {
            self.reverse_lights = self.reverse;
        } else {
            self.reverse_lights = false;
        }
    }

    pub fn force_reverse_lights(&mut self, on: bool) {
        self.reverse_lights = on;
    }

    pub fn force_reverse_camera(&mut self, on: bool) {
        self.reverse_camera = on;
    }

    pub fn reverse(&mut self, on: bool) {
        self.reverse = on;

        if !self.reverse {
            self.reverse_lights = false;
            self.reverse_camera = false;
        } else {
            self.reverse_camera = true;
            if self.reverse_lights_active {
                self.reverse_lights = true;
            }
        }
    }

    pub fn high_beam(&mut self, on: bool) {
        self.high_beam = on;

        if self.high_beam {
            if self.led_bar_active {
                self.led_bar = true;
            }
        } else {
            self.led_bar = false;
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
