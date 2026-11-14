//! Serial ports: the list with USB adapters first, and opening one with the line's settings.
//! Windows and Linux; Android has no serial port for a program (concept section 10).

use serde::Serialize;

use crate::endpoint::SerialSettings;
use crate::link::{ByteStream, LinkError};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PortChoice {
    pub name: String,
    /// The adapter's own name, where the system knows it ("USB Serial (FTDI)").
    pub description: Option<String>,
    pub usb: bool,
}

#[cfg(not(target_os = "android"))]
pub fn ports() -> Result<Vec<PortChoice>, String> {
    let ports = tokio_serial::available_ports()
        .map_err(|error| format!("cannot list the serial ports: {error}"))?;
    let mut choices: Vec<PortChoice> = ports
        .into_iter()
        .map(|port| match port.port_type {
            tokio_serial::SerialPortType::UsbPort(usb) => PortChoice {
                name: port.port_name,
                description: usb.product.or(usb.manufacturer),
                usb: true,
            },
            _ => PortChoice {
                name: port.port_name,
                description: None,
                usb: false,
            },
        })
        .filter(|choice| choice.usb || !placeholder(&choice.name))
        .collect();
    sort(&mut choices);
    Ok(choices)
}

/// A port Linux names without a chip behind it: `ttyS0` to `ttyS3` exist on every PC, and the
/// kernel gives their type as 0 (no UART) where there is none. Nothing can be connected there.
#[cfg(target_os = "linux")]
fn placeholder(name: &str) -> bool {
    let Some(device) = name.rsplit('/').next() else {
        return false;
    };
    no_uart(
        std::fs::read_to_string(format!("/sys/class/tty/{device}/type"))
            .ok()
            .as_deref(),
    )
}

#[cfg(not(target_os = "linux"))]
fn placeholder(_name: &str) -> bool {
    false
}

/// The kernel's type of a serial port's chip, as `/sys/class/tty/*/type` gives it: 0 is none.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn no_uart(type_text: Option<&str>) -> bool {
    type_text.map(str::trim) == Some("0")
}

#[cfg(target_os = "android")]
pub fn ports() -> Result<Vec<PortChoice>, String> {
    Ok(Vec::new())
}

#[cfg(not(target_os = "android"))]
fn sort(choices: &mut [PortChoice]) {
    choices.sort_by(|a, b| b.usb.cmp(&a.usb).then_with(|| a.name.cmp(&b.name)));
}

#[cfg(not(target_os = "android"))]
pub fn open(settings: &SerialSettings) -> Result<Box<dyn ByteStream>, LinkError> {
    use crate::endpoint::Parity;
    use tokio_serial::SerialPortBuilderExt;

    let parity = match settings.parity {
        Parity::None => tokio_serial::Parity::None,
        Parity::Even => tokio_serial::Parity::Even,
        Parity::Odd => tokio_serial::Parity::Odd,
    };
    let stop_bits = if settings.stop_bits == 2 {
        tokio_serial::StopBits::Two
    } else {
        tokio_serial::StopBits::One
    };
    tokio_serial::new(&settings.port, settings.baud)
        .data_bits(tokio_serial::DataBits::Eight)
        .parity(parity)
        .stop_bits(stop_bits)
        .flow_control(tokio_serial::FlowControl::None)
        .open_native_async()
        .map(|port| Box::new(port) as Box<dyn ByteStream>)
        .map_err(|error| LinkError::Open(open_error_text(&settings.port, &error)))
}

#[cfg(target_os = "android")]
pub fn open(_settings: &SerialSettings) -> Result<Box<dyn ByteStream>, LinkError> {
    Err(LinkError::Open("this system has no serial port".into()))
}

#[cfg(not(target_os = "android"))]
fn open_error_text(port: &str, error: &tokio_serial::Error) -> String {
    let text = error.to_string();
    if text.contains("Permission denied") || text.contains("permission") {
        return format!(
            "no permission to open {port}: add yourself to the group dialout \
             (sudo usermod -aG dialout $USER) and log in again"
        );
    }
    if text.contains("busy") || text.contains("Access is denied") || text.contains("in use") {
        return format!("{port} is used by another program; close it there first");
    }
    if text.contains("No such file") || text.contains("not find") {
        return format!("{port} does not exist (any more): is the adapter plugged in?");
    }
    format!("cannot open {port}: {text}")
}

#[cfg(all(test, not(target_os = "android")))]
mod tests {
    use super::*;

    #[test]
    fn usb_adapters_come_first() {
        let port = |name: &str, usb| PortChoice {
            name: name.into(),
            description: None,
            usb,
        };
        let mut choices = vec![
            port("/dev/ttyS0", false),
            port("/dev/ttyUSB1", true),
            port("/dev/ttyUSB0", true),
        ];
        sort(&mut choices);
        let names: Vec<&str> = choices.iter().map(|choice| choice.name.as_str()).collect();
        assert_eq!(names, ["/dev/ttyUSB0", "/dev/ttyUSB1", "/dev/ttyS0"]);
    }

    #[test]
    fn a_port_without_a_chip_is_told_by_its_type() {
        assert!(no_uart(Some("0\n")));
        // 4 is a 16550A, the usual chip
        assert!(!no_uart(Some("4\n")));
        // no type to read: a USB adapter or a system without that file, kept
        assert!(!no_uart(None));
    }
}
