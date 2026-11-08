//! Where a device is reached: a TCP address, a serial gateway passing RTU frames over TCP, or a
//! serial port with its settings. Stored as text: `tcp:10.0.0.5:502`, `rtu-tcp:10.0.0.9:4001`,
//! `serial:COM3:9600:8E1`.

use std::fmt;
use std::net::SocketAddr;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::protocol::tcp::MODBUS_PORT;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Parity {
    None,
    Even,
    Odd,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SerialSettings {
    pub port: String,
    pub baud: u32,
    pub parity: Parity,
    pub stop_bits: u8,
}

/// Speeds a serial search tries, the usual ones first.
pub const BAUD_RATES: [u32; 8] = [9600, 19200, 38400, 4800, 2400, 57600, 115200, 1200];

impl SerialSettings {
    /// `8E1`, `8N2` …
    pub fn frame_text(&self) -> String {
        let parity = match self.parity {
            Parity::None => 'N',
            Parity::Even => 'E',
            Parity::Odd => 'O',
        };
        format!("8{parity}{}", self.stop_bits)
    }

    pub fn parse_frame(text: &str) -> Option<(Parity, u8)> {
        let bytes = text.trim().to_ascii_uppercase().into_bytes();
        let [b'8', parity, stop] = bytes[..] else {
            return None;
        };
        let parity = match parity {
            b'N' => Parity::None,
            b'E' => Parity::Even,
            b'O' => Parity::Odd,
            _ => return None,
        };
        let stop = match stop {
            b'1' => 1,
            b'2' => 2,
            _ => return None,
        };
        Some((parity, stop))
    }

    /// The frames a search tries at each speed: the spec's 8E1 first, then what devices use too.
    pub fn frames() -> [(Parity, u8); 4] {
        [
            (Parity::Even, 1),
            (Parity::None, 1),
            (Parity::None, 2),
            (Parity::Odd, 1),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Endpoint {
    Tcp(SocketAddr),
    /// A serial gateway in transparent mode: RTU frames with CRC inside a TCP connection.
    RtuOverTcp(SocketAddr),
    Serial(SerialSettings),
}

impl Endpoint {
    pub fn is_serial_line(&self) -> bool {
        matches!(self, Endpoint::Serial(_) | Endpoint::RtuOverTcp(_))
    }

    /// The same line regardless of settings: one serial port is one line at any speed.
    pub fn line_key(&self) -> String {
        match self {
            Endpoint::Serial(settings) => format!("serial:{}", settings.port),
            other => other.to_string(),
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Endpoint::Tcp(address) => write!(f, "tcp:{address}"),
            Endpoint::RtuOverTcp(address) => write!(f, "rtu-tcp:{address}"),
            Endpoint::Serial(settings) => write!(
                f,
                "serial:{}:{}:{}",
                settings.port,
                settings.baud,
                settings.frame_text()
            ),
        }
    }
}

impl FromStr for Endpoint {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let address = |rest: &str| -> Result<SocketAddr, String> {
            let with_port = if rest.contains(':') {
                rest.to_string()
            } else {
                format!("{rest}:{MODBUS_PORT}")
            };
            with_port
                .parse()
                .map_err(|_| format!("{rest} is not an IP address with an optional port"))
        };
        if let Some(rest) = text.strip_prefix("rtu-tcp:") {
            return Ok(Endpoint::RtuOverTcp(address(rest)?));
        }
        if let Some(rest) = text.strip_prefix("tcp:") {
            return Ok(Endpoint::Tcp(address(rest)?));
        }
        if let Some(rest) = text.strip_prefix("serial:") {
            // the port may hold colons on no system we know, but split from the right to be safe
            let mut parts = rest.rsplitn(3, ':');
            let frame = parts.next().unwrap_or_default();
            let baud = parts.next().unwrap_or_default();
            let port = parts.next().unwrap_or_default();
            let (parity, stop_bits) = SerialSettings::parse_frame(frame)
                .ok_or_else(|| format!("{frame} is not a frame like 8E1"))?;
            let baud: u32 = baud
                .parse()
                .ok()
                .filter(|baud| (300..=1_000_000).contains(baud))
                .ok_or_else(|| format!("{baud} is not a speed"))?;
            if port.is_empty() {
                return Err("the serial port is missing".into());
            }
            return Ok(Endpoint::Serial(SerialSettings {
                port: port.into(),
                baud,
                parity,
                stop_bits,
            }));
        }
        Ok(Endpoint::Tcp(address(text)?))
    }
}

impl Serialize for Endpoint {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Endpoint {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// A device: where it is reached and its unit ID.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Target {
    pub endpoint: Endpoint,
    pub unit: u8,
}

impl Target {
    /// `tcp:10.0.0.5:502/1`, the key of a device in files and the API.
    pub fn key(&self) -> String {
        format!("{}/{}", self.endpoint, self.unit)
    }

    pub fn parse_key(text: &str) -> Result<Target, String> {
        let (endpoint, unit) = text
            .rsplit_once('/')
            .ok_or_else(|| format!("{text} has no unit ID after a /"))?;
        let unit = unit
            .parse()
            .map_err(|_| format!("{unit} is not a unit ID 0 to 255"))?;
        Ok(Target {
            endpoint: endpoint.parse()?,
            unit,
        })
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.endpoint {
            Endpoint::Tcp(address) => write!(f, "{address} unit {}", self.unit),
            Endpoint::RtuOverTcp(address) => write!(f, "{address} (RTU) unit {}", self.unit),
            Endpoint::Serial(settings) => write!(
                f,
                "{} {} {} unit {}",
                settings.port,
                settings.baud,
                settings.frame_text(),
                self.unit
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_go_to_text_and_back() {
        for text in [
            "tcp:10.0.0.5:502",
            "rtu-tcp:10.0.0.9:4001",
            "serial:/dev/ttyUSB0:9600:8E1",
            "serial:COM3:19200:8N2",
        ] {
            let endpoint: Endpoint = text.parse().expect("parses");
            assert_eq!(endpoint.to_string(), text);
        }
        assert_eq!(
            "10.0.0.5".parse::<Endpoint>(),
            Ok(Endpoint::Tcp("10.0.0.5:502".parse().expect("address")))
        );
        assert!("serial:COM3:9600:7E1".parse::<Endpoint>().is_err());
        assert!("serial:COM3:fast:8E1".parse::<Endpoint>().is_err());
    }

    #[test]
    fn a_device_key_holds_endpoint_and_unit() {
        let target = Target::parse_key("serial:COM3:9600:8E1/12").expect("parses");
        assert_eq!(target.unit, 12);
        assert_eq!(target.key(), "serial:COM3:9600:8E1/12");
        assert!(Target::parse_key("tcp:10.0.0.5:502").is_err());
    }
}
