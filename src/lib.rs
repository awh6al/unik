//! This crate defines a powerful uniform resource name namespace for UUIDs
//! (Universally Unique Identifier), which are suited for modern use.
//!
//! This lib can be used to create unique and reasonably short values without
//! requiring extra knowledge.
//!
//! A `UUID` is 128 bits long, and can guarantee uniqueness across space and time.
#![doc(html_root_url = "https://docs.rs/unik")]
#![feature(doc_cfg)]

pub mod rfc4122;

use core::fmt;
use std::sync::atomic::{self, AtomicU16};

pub use mac_address::{get_mac_address, MacAddress};

/// Represent bytes of MAC address.
pub type Node = MacAddress;

/// Is a 60-bit value. Represented by Coordinated Universal Time (UTC).
///
/// NOTE: `TimeStamp` used as a `u64`. For this reason dates prior to gregorian
/// calendar are not supported.
#[derive(Debug, Clone, Copy)]
pub struct TimeStamp(pub u64);

/// The simplified version of `UUID` in terms of fields that are integral numbers of octets.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Layout {
    /// The low field of the TimeStamp.
    field_low: u32,
    /// The mid field of the TimeStamp.
    field_mid: u16,
    /// The high field of the TimeStamp multiplexed with the version number.
    field_high_and_version: u16,
    /// The high field of the ClockSeq multiplexed with the variant.
    clock_seq_high_and_reserved: u8,
    /// The low field of the ClockSeq.
    clock_seq_low: u8,
    /// IEEE-802 network address.
    node: Node,
}

impl Layout {
    // Returns `Layout` from sequence of integral numbers.
    pub fn from_bytes(bytes: Bytes) -> Layout {
        let field_low = u32::from_le_bytes([bytes[3], bytes[2], bytes[1], bytes[0]]);
        let field_mid = u16::from_le_bytes([bytes[5], bytes[4]]);
        let field_high_and_version = u16::from_le_bytes([bytes[7], bytes[6]]);
        let clock_seq_high_and_reserved = bytes[8];
        let clock_seq_low = bytes[9];

        Layout {
            field_low,
            field_mid,
            field_high_and_version,
            clock_seq_high_and_reserved,
            clock_seq_low,
            node: MacAddress::new([
                bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
            ]),
        }
    }

    /// Returns the memory representation of `UUID`.
    pub fn generate(&self) -> UUID {
        UUID([
            self.field_low.to_le_bytes()[3],
            self.field_low.to_le_bytes()[2],
            self.field_low.to_le_bytes()[1],
            self.field_low.to_le_bytes()[0],
            self.field_mid.to_le_bytes()[1],
            self.field_mid.to_le_bytes()[0],
            self.field_high_and_version.to_le_bytes()[1],
            self.field_high_and_version.to_le_bytes()[0],
            self.clock_seq_high_and_reserved,
            self.clock_seq_low,
            self.node.bytes()[0],
            self.node.bytes()[1],
            self.node.bytes()[2],
            self.node.bytes()[3],
            self.node.bytes()[4],
            self.node.bytes()[5],
        ])
    }

    /// Returns the algorithm number of `UUID`.
    pub const fn get_version(&self) -> Result<Version, &str> {
        match (self.field_high_and_version) >> 12 {
            0x1 => Ok(Version::TIME),
            0x2 => Ok(Version::DCE),
            0x3 => Ok(Version::MD5),
            0x4 => Ok(Version::RAND),
            0x5 => Ok(Version::SHA1),
            _ => Err("Invalid version"),
        }
    }

    /// Returns the type field of `UUID`.
    pub const fn get_variant(&self) -> Result<Variant, &str> {
        match self.clock_seq_high_and_reserved >> 0x4 {
            0x0 => Ok(Variant::NCS),
            0x1 => Ok(Variant::RFC4122),
            0x2 => Ok(Variant::MS),
            0x3 => Ok(Variant::FUT),
            _ => Err("Invalid variant"),
        }
    }
}

/// The `UUID` format is 16 octets.
pub type Bytes = [u8; 16];

/// Is a 128-bit number used to identify information in computer systems.
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct UUID(Bytes);

impl UUID {
    /// Returns the memory representation of `UUID`.
    pub const fn as_bytes(&self) -> [u8; 16] {
        self.0
    }

    /// UUID namespace for domain name system (DNS).
    pub const NAMESPACE_DNS: UUID = UUID([
        0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30,
        0xc8,
    ]);

    /// UUID namespace for ISO object identifiers (OIDs).
    pub const NAMESPACE_OID: UUID = UUID([
        0x6b, 0xa7, 0xb8, 0x12, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30,
        0xc8,
    ]);

    /// UUID namespace for uniform resource locators (URLs).
    pub const NAMESPACE_URL: UUID = UUID([
        0x6b, 0xa7, 0xb8, 0x11, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30,
        0xc8,
    ]);

    /// UUID namespace for X.500 distinguished names (DNs).
    pub const NAMESPACE_X500: UUID = UUID([
        0x6b, 0xa7, 0xb8, 0x14, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30,
        0xc8,
    ]);

    // Parse `UUID` from a string of hex digits.
    pub fn from_str(us: &str) -> Result<Layout, &str> {
        let mut us = us.to_string();
        let mut bytes = [0; 16];

        if us.len() == 36 || us.len() == 32 {
            if us.contains('-') {
                us.retain(|c| !c.is_ascii_whitespace() && c != '-');
            }

            for i in 0..15 {
                let s = &us[i * 2..i * 2 + 2];
                let byte = u8::from_str_radix(s, 16).map_err(|_| "Invalid UUID string")?;

                bytes[i] = byte;
            }
        } else {
            return Err("Invalid UUID string");
        }

        Ok(layout!(
            bytes[3], bytes[2], bytes[1], bytes[0], bytes[5], bytes[4], bytes[7], bytes[6],
            bytes[9], bytes[8], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
        ))
    }
}

impl fmt::Display for UUID {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
                fmt,
                "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                self.0[0],
                self.0[1],
                self.0[2],
                self.0[3],
                self.0[4],
                self.0[5],
                self.0[6],
                self.0[7],
                self.0[8],
                self.0[9],
                self.0[10],
                self.0[11],
                self.0[12],
                self.0[13],
                self.0[14],
                self.0[15],
            )
    }
}

impl fmt::LowerHex for UUID {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.0[0],
            self.0[1],
            self.0[2],
            self.0[3],
            self.0[4],
            self.0[5],
            self.0[6],
            self.0[7],
            self.0[8],
            self.0[9],
            self.0[10],
            self.0[11],
            self.0[12],
            self.0[13],
            self.0[14],
            self.0[15],
        )
    }
}

impl fmt::UpperHex for UUID {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            self.0[0],
            self.0[1],
            self.0[2],
            self.0[3],
            self.0[4],
            self.0[5],
            self.0[6],
            self.0[7],
            self.0[8],
            self.0[9],
            self.0[10],
            self.0[11],
            self.0[12],
            self.0[13],
            self.0[14],
            self.0[15],
        )
    }
}

/// Represents the algorithm use for building the `Layout`, located in
/// the most significant 4 bits of `TimeStamp`.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Version {
    /// The time-based version specified in `rfc4122` document.
    TIME = 1,
    /// DCE-security version, with embedded POSIX UIDs.
    DCE,
    /// The name-based version specified in `rfc4122` document that uses MD5 hashing.
    MD5,
    /// The randomly or pseudo-randomly generated version specified in `rfc4122` document.
    RAND,
    /// The name-based version specified in `rfc4122`document that uses SHA1 hashing.
    SHA1,
}

impl std::string::ToString for Version {
    fn to_string(&self) -> String {
        match self {
            Version::TIME => "TIME".to_owned(),
            Version::DCE => "DCE".to_owned(),
            Version::MD5 => "MD5".to_owned(),
            Version::RAND => "RAND".to_owned(),
            Version::SHA1 => "SHA1".to_owned(),
        }
    }
}

/// Is a type field determines the layout of `UUID`.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Variant {
    /// Reserved, NCS backward compatibility.
    NCS = 0,
    /// The variant specified in `rfc4122` document.
    RFC4122,
    /// Reserved, Microsoft Corporation backward compatibility.
    MS,
    /// Reserved for future definition.
    FUT,
}

impl std::string::ToString for Variant {
    fn to_string(&self) -> String {
        match self {
            Variant::NCS => "NCS".to_owned(),
            Variant::RFC4122 => "RFC".to_owned(),
            Variant::MS => "MS".to_owned(),
            Variant::FUT => "FUT".to_owned(),
        }
    }
}

/// Used to avoid duplicates that could arise when the clock is set backwards in time.
pub struct ClockSeq(u16);

impl ClockSeq {
    pub fn new(rand: u16) -> u16 {
        AtomicU16::new(rand).fetch_add(1, atomic::Ordering::SeqCst)
    }
}

#[macro_export]
macro_rules! layout {
    ($b0:expr, $b1:expr, $b2:expr, $b3:expr,
        $b4:expr, $b5:expr, $b6:expr, $b7:expr,
        $b8:expr, $b9:expr, $b10:expr, $b11:expr,
        $b12:expr, $b13:expr, $b14:expr, $b15:expr) => {
        Layout {
            field_low: $b0 as u32 | ($b1 as u32) << 8 | ($b2 as u32) << 16 | ($b3 as u32) << 24,
            field_mid: ($b4 as u16) | (($b5 as u16) << 8),
            field_high_and_version: ($b6 as u16) | ($b7 as u16) << 8,
            clock_seq_high_and_reserved: ($b8 & 0xf) as u8 | (Variant::RFC4122 as u8) << 4,
            clock_seq_low: $b9,
            node: MacAddress::new([$b10, $b11, $b12, $b13, $b14, $b15]),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_default() {
        let uuid = UUID::default();
        assert_eq!(uuid, UUID([0; 16]));
    }

    #[test]
    fn parse_string() {
        let cols = [
            ("ab720268-b83f-11ec-b909-0242ac120002", Version::TIME),
            ("000003e8-c22b-21ec-bd01-d4bed9408ecc", Version::DCE),
            ("2448bd95-00ca-3650-160f-3301a691b26c", Version::MD5),
            ("6a665038-24cf-4cf6-9b61-05f0c2fc6c08", Version::RAND),
            ("991da866-83b0-5550-1bef-37a1a5b1fb30", Version::SHA1),
        ];

        for item in cols {
            assert_eq!(UUID::from_str(item.0).unwrap().get_version(), Ok(item.1));
            assert_eq!(
                UUID::from_str(item.0).unwrap().get_variant(),
                Ok(Variant::RFC4122)
            );
        }
    }
}
