//! Simulated Modbus devices for the examples and tests, served by tokio-modbus: an implementation
//! independent of modbusio's own, so both sides of every test are not the same code. Names and
//! numbers are made up.

#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio_modbus::prelude::*;

pub mod trunk;

/// One simulated device: its four areas, which addresses exist, what it refuses.
#[derive(Clone, Default)]
pub struct Device {
    pub name: &'static str,
    pub coils: BTreeMap<u16, bool>,
    pub discrete: BTreeMap<u16, bool>,
    pub input: BTreeMap<u16, u16>,
    pub holding: BTreeMap<u16, u16>,
    /// Holding registers that refuse a write with exception 03, as many devices do.
    pub read_only: Vec<u16>,
    /// Most registers per request; more is refused with exception 03.
    pub most_registers: u16,
    /// Read Device Identification objects (43/14), where the device offers it.
    pub identity: Vec<(u8, &'static str)>,
    /// Refuses function 16 for one register (exception 01): such devices want 06.
    pub single_writes_only: bool,
    /// Time before every answer.
    pub delay: Duration,
}

/// The addresses of a request. After 65535 there is none: the device refuses with exception 02.
fn span(address: u16, count: usize) -> impl Iterator<Item = Result<u16, ExceptionCode>> {
    (0..count).map(move |offset| {
        u16::try_from(offset)
            .ok()
            .and_then(|offset| address.checked_add(offset))
            .ok_or(ExceptionCode::IllegalDataAddress)
    })
}

impl Device {
    fn read_registers(
        &self,
        area: &BTreeMap<u16, u16>,
        address: u16,
        count: u16,
    ) -> Result<Vec<u16>, ExceptionCode> {
        if count > self.most_registers.max(1) {
            return Err(ExceptionCode::IllegalDataValue);
        }
        span(address, usize::from(count))
            .map(|at| {
                at.and_then(|at| {
                    area.get(&at)
                        .copied()
                        .ok_or(ExceptionCode::IllegalDataAddress)
                })
            })
            .collect()
    }

    fn read_bits(
        area: &BTreeMap<u16, bool>,
        address: u16,
        count: u16,
    ) -> Result<Vec<bool>, ExceptionCode> {
        span(address, usize::from(count))
            .map(|at| {
                at.and_then(|at| {
                    area.get(&at)
                        .copied()
                        .ok_or(ExceptionCode::IllegalDataAddress)
                })
            })
            .collect()
    }

    pub fn answer(&mut self, request: Request<'static>) -> Result<Response, ExceptionCode> {
        match request {
            Request::ReadCoils(address, count) => {
                Device::read_bits(&self.coils, address, count).map(Response::ReadCoils)
            }
            Request::ReadDiscreteInputs(address, count) => {
                Device::read_bits(&self.discrete, address, count).map(Response::ReadDiscreteInputs)
            }
            Request::ReadInputRegisters(address, count) => self
                .read_registers(&self.input, address, count)
                .map(Response::ReadInputRegisters),
            Request::ReadHoldingRegisters(address, count) => self
                .read_registers(&self.holding, address, count)
                .map(Response::ReadHoldingRegisters),
            Request::WriteSingleRegister(address, value) => {
                self.write_registers(address, &[value])?;
                Ok(Response::WriteSingleRegister(address, value))
            }
            Request::WriteMultipleRegisters(address, values) => {
                if self.single_writes_only && values.len() == 1 {
                    return Err(ExceptionCode::IllegalFunction);
                }
                self.write_registers(address, &values)?;
                Ok(Response::WriteMultipleRegisters(
                    address,
                    values.len() as u16,
                ))
            }
            Request::WriteSingleCoil(address, value) => {
                self.write_coils(address, &[value])?;
                Ok(Response::WriteSingleCoil(address, value))
            }
            Request::WriteMultipleCoils(address, values) => {
                self.write_coils(address, &values)?;
                Ok(Response::WriteMultipleCoils(address, values.len() as u16))
            }
            Request::ReadDeviceIdentification(code, first) => self.identify(code, first),
            _ => Err(ExceptionCode::IllegalFunction),
        }
    }

    fn write_registers(&mut self, address: u16, values: &[u16]) -> Result<(), ExceptionCode> {
        let places: Vec<u16> = span(address, values.len()).collect::<Result<_, _>>()?;
        for at in &places {
            if !self.holding.contains_key(at) {
                return Err(ExceptionCode::IllegalDataAddress);
            }
            if self.read_only.contains(at) {
                return Err(ExceptionCode::IllegalDataValue);
            }
        }
        self.holding
            .extend(places.into_iter().zip(values.iter().copied()));
        Ok(())
    }

    fn write_coils(&mut self, address: u16, values: &[bool]) -> Result<(), ExceptionCode> {
        let places: Vec<u16> = span(address, values.len()).collect::<Result<_, _>>()?;
        if places.iter().any(|at| !self.coils.contains_key(at)) {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        self.coils
            .extend(places.into_iter().zip(values.iter().copied()));
        Ok(())
    }

    fn identify(&self, code: ReadCode, first: u8) -> Result<Response, ExceptionCode> {
        if self.identity.is_empty() {
            return Err(ExceptionCode::IllegalFunction);
        }
        let objects = self
            .identity
            .iter()
            .filter(|(id, _)| *id >= first)
            .map(|(id, text)| DeviceIdObject {
                id: *id,
                value: text.as_bytes().to_vec().into(),
            })
            .collect();
        Ok(Response::ReadDeviceIdentification(
            ReadDeviceIdentificationResponse {
                read_code: code,
                conformity_level: ConformityLevel::BasicIdentificationStreamOnly,
                more_follows: false,
                next_object_id: 0,
                device_id_objects: objects,
            },
        ))
    }

    fn set_f32(registers: &mut BTreeMap<u16, u16>, address: u16, value: f32, swapped: bool) {
        let bits = value.to_bits();
        let (high, low) = ((bits >> 16) as u16, (bits & 0xFFFF) as u16);
        let (first, second) = if swapped { (low, high) } else { (high, low) };
        registers.insert(address, first);
        registers.insert(address + 1, second);
    }

    fn set_u32(registers: &mut BTreeMap<u16, u16>, address: u16, value: u32) {
        registers.insert(address, (value >> 16) as u16);
        registers.insert(address + 1, (value & 0xFFFF) as u16);
    }
}

fn text_registers(text: &str, registers: usize) -> Vec<u16> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.resize(registers * 2, 0);
    bytes
        .chunks(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect()
}

/// An electricity meter on TCP: floats in ABCD in the input registers, energy as u32 Wh, two
/// holding registers for its settings, identification.
pub fn energy_meter() -> Device {
    let mut device = Device {
        name: "EM-340 energy meter",
        most_registers: 125,
        identity: vec![(0, "Simulated Meters"), (1, "EM-340"), (2, "2.1.0")],
        ..Device::default()
    };
    for (address, value) in [
        (0, 230.1f32),
        (2, 229.8),
        (4, 231.0),
        (6, 5.12),
        (8, 4.98),
        (10, 5.30),
        (12, 3.52),
        (14, 50.0),
    ] {
        Device::set_f32(&mut device.input, address, value, false);
    }
    Device::set_u32(&mut device.input, 20, 1_234_567);
    // settings: CT ratio and the Modbus address shown on the display
    device.holding.insert(100, 50);
    device.holding.insert(101, 1);
    device.read_only.push(101);
    device
}

/// A heat meter behind a gateway: floats words swapped (CDAB), a volume with a scale register,
/// a serial number as text.
pub fn heat_meter() -> Device {
    let mut device = Device {
        name: "HM-50 heat meter",
        most_registers: 64,
        ..Device::default()
    };
    Device::set_f32(&mut device.holding, 0, 1234.5, true); // energy kWh
    Device::set_f32(&mut device.holding, 2, 63.2, true); // flow temperature
    Device::set_f32(&mut device.holding, 4, 41.0, true); // return temperature
    Device::set_f32(&mut device.holding, 6, 0.85, true); // flow m³/h
    device.holding.insert(8, 8821); // volume, × 10^scale
    device.holding.insert(9, (-2i16) as u16); // scale of the volume
    for (offset, register) in text_registers("HM5012345678", 6).into_iter().enumerate() {
        device.holding.insert(20 + offset as u16, register);
    }
    device
}

/// A drive: holding registers with gaps between blocks, at most 16 registers per request, coils
/// for run and reset, a fault code; refuses 16 for a single register.
pub fn drive() -> Device {
    let mut device = Device {
        name: "FU-7 drive",
        most_registers: 16,
        single_writes_only: true,
        ..Device::default()
    };
    device.holding.insert(0, 0); // command word
    device.holding.insert(1, 2500); // set frequency × 0.01 Hz
    device.holding.insert(2, 300); // ramp up × 0.1 s
    device.holding.insert(3, 300); // ramp down × 0.1 s
    // a gap 4..99
    device.holding.insert(100, 2498); // output frequency × 0.01 Hz
    device.holding.insert(101, 123); // current × 0.1 A
    device.holding.insert(102, 4000); // DC bus voltage × 0.1 V
    device.holding.insert(103, 0); // fault code
    device.read_only.extend([100, 101, 102, 103]);
    device.coils.insert(0, false); // run
    device.coils.insert(1, false); // reset fault
    device.discrete.insert(0, true); // ready
    device.discrete.insert(1, false); // fault
    device
}

/// A room controller: temperatures as s16 × 0.1, a mode with state numbers, a set point.
pub fn room_controller() -> Device {
    let mut device = Device {
        name: "RC-10 room controller",
        most_registers: 125,
        ..Device::default()
    };
    device.input.insert(0, 215); // room temperature × 0.1 °C
    device.input.insert(1, (-35i16) as u16); // outdoor temperature × 0.1 °C
    device.input.insert(2, 45); // humidity %
    device.holding.insert(0, 1); // mode: 0 off, 1 heating, 2 cooling
    device.holding.insert(1, 210); // set point × 0.1 °C
    device
}

/// Devices by unit ID, shared between connections; values drift so live views change.
#[derive(Clone)]
pub struct Units {
    devices: Arc<Mutex<HashMap<u8, Device>>>,
    /// Answers every unit ID with the device of the first (a direct TCP device does).
    any_unit: bool,
    /// Answers units it does not have with exception 0B after this time (a gateway does).
    gateway_delay: Option<Duration>,
    started: Instant,
}

impl Units {
    pub fn new(devices: Vec<(u8, Device)>) -> Units {
        Units {
            devices: Arc::new(Mutex::new(devices.into_iter().collect())),
            any_unit: false,
            gateway_delay: None,
            started: Instant::now(),
        }
    }

    pub fn answering_any_unit(mut self) -> Units {
        self.any_unit = true;
        self
    }

    pub fn behind_gateway(mut self, delay: Duration) -> Units {
        self.gateway_delay = Some(delay);
        self
    }

    fn drift(&self, device: &mut Device) {
        let seconds = self.started.elapsed().as_secs_f32();
        let wave = (seconds / 7.0).sin();
        if device.name.starts_with("RC-10") {
            device.input.insert(0, (215.0 + wave * 6.0) as i16 as u16);
        }
        if device.name.starts_with("EM-340") {
            Device::set_f32(&mut device.input, 6, 5.12 + wave * 0.4, false);
            Device::set_u32(&mut device.input, 20, 1_234_567 + seconds as u32);
        }
        if device.name.starts_with("HM-50") {
            Device::set_f32(&mut device.holding, 2, 63.2 + wave, true);
        }
    }

    /// The answer for `unit`; `None` means silence (no such unit on a line).
    pub fn answer(
        &self,
        unit: u8,
        request: Request<'static>,
    ) -> (Duration, Option<Result<Response, ExceptionCode>>) {
        let mut devices = self.devices.lock().expect("devices lock");
        let key = if self.any_unit {
            devices.keys().next().copied()
        } else {
            Some(unit)
        };
        match key.and_then(|key| devices.get_mut(&key)) {
            Some(device) => {
                self.drift(device);
                (device.delay, Some(device.answer(request)))
            }
            None => match self.gateway_delay {
                Some(delay) => (delay, Some(Err(ExceptionCode::GatewayTargetDevice))),
                None => (Duration::ZERO, None),
            },
        }
    }
}

impl tokio_modbus::server::Service for Units {
    type Request = SlaveRequest<'static>;
    type Response = Option<Response>;
    type Exception = ExceptionCode;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Exception>> + Send>>;

    fn call(&self, request: Self::Request) -> Self::Future {
        let (delay, answer) = self.answer(request.slave, request.request);
        Box::pin(async move {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            match answer {
                None => Ok(None),
                Some(Ok(response)) => Ok(Some(response)),
                Some(Err(exception)) => Err(exception),
            }
        })
    }
}

/// Serves `units` as Modbus TCP on `listener` until the process ends. With `one_connection`, a
/// second connection is closed at once, as many small devices do.
pub async fn serve_tcp(
    listener: tokio::net::TcpListener,
    units: Units,
    one_connection: bool,
) -> std::io::Result<()> {
    use tokio_modbus::server::tcp::Server;
    let server = Server::new(listener);
    let open = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let on_connected = move |stream: tokio::net::TcpStream, _socket: std::net::SocketAddr| {
        let units = units.clone();
        let open = open.clone();
        async move {
            if one_connection && open.load(std::sync::atomic::Ordering::SeqCst) > 0 {
                return Ok(None);
            }
            open.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(Some((
                units,
                Tracked {
                    inner: stream,
                    open,
                },
            )))
        }
    };
    server
        .serve(&on_connected, |error| {
            eprintln!("connection ended: {error}")
        })
        .await
}

/// Serves `units` as RTU frames inside TCP (a serial gateway in transparent mode).
pub async fn serve_rtu_over_tcp(
    listener: tokio::net::TcpListener,
    units: Units,
) -> std::io::Result<()> {
    use tokio_modbus::server::rtu_over_tcp::{Server, accept_tcp_connection};
    let server = Server::new(listener);
    let on_connected = move |stream: tokio::net::TcpStream, socket: std::net::SocketAddr| {
        let units = units.clone();
        async move { accept_tcp_connection(stream, socket, move |_| Ok(Some(units.clone()))) }
    };
    server
        .serve(&on_connected, |error| {
            eprintln!("connection ended: {error}")
        })
        .await
}

/// A TCP stream that counts itself out of the open connections when dropped.
struct Tracked {
    inner: tokio::net::TcpStream,
    open: Arc<std::sync::atomic::AtomicUsize>,
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.open.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

impl tokio::io::AsyncRead for Tracked {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for Tracked {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }
    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}
