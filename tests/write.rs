//! Writing against a device that behaves badly: the log says what the device holds, not what
//! modbusio hoped for.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

use modbusio::access::Limits;
use modbusio::endpoint::Endpoint;
use modbusio::link::LinkError;
use modbusio::protocol::Area;
use modbusio::write::write;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A TCP device with one holding register, answering reads. It takes a write (16) of
/// `taken_silently` without confirming it and stays silent for any other.
async fn device_confirming_no_write(register: Arc<AtomicU16>, taken_silently: u16) -> SocketAddr {
    let (listener, address) = common::listener().await;
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("the master");
        let mut request = [0u8; 260];
        while let Ok(length @ 1..) = stream.read(&mut request).await {
            let request = &request[..length];
            if request[7] == 0x03 {
                let mut answer = request[..4].to_vec();
                answer.extend([0, 5, request[6], 0x03, 2]);
                answer.extend(register.load(Ordering::SeqCst).to_be_bytes());
                stream.write_all(&answer).await.expect("the answer");
                continue;
            }
            let value = u16::from_be_bytes([request[13], request[14]]);
            if value == taken_silently {
                register.store(value, Ordering::SeqCst);
            }
        }
    });
    address
}

#[tokio::test]
async fn a_write_without_confirmation_is_read_back() {
    let register = Arc::new(AtomicU16::new(2500));
    let address = device_confirming_no_write(register.clone(), 3000).await;
    let links = common::links();
    let target = common::target(Endpoint::Tcp(address), 1);
    let wait = Duration::from_millis(150);
    let area = Area::HoldingRegister;
    let mut limits = Limits::default();

    // the device took the value and the confirmation was lost: the write stands
    let taken = write(&links, &target, area, 0, &[3000], &mut limits, wait).await;
    assert_eq!(taken.before, Some(vec![2500]));
    assert_eq!(taken.after, Some(vec![3000]));
    assert_eq!(taken.error, None);

    // the device did not take it: the write failed, and the log shows what the device holds
    let lost = write(&links, &target, area, 0, &[4000], &mut limits, wait).await;
    assert_eq!(lost.before, Some(vec![3000]));
    assert_eq!(lost.after, Some(vec![3000]));
    assert_eq!(lost.error, Some(LinkError::Timeout));

    // writing the value the device holds already proves nothing without a confirmation
    let same = write(&links, &target, area, 0, &[3000], &mut limits, wait).await;
    assert_eq!(same.error, Some(LinkError::Timeout));
    assert_eq!(register.load(Ordering::SeqCst), 3000);
}
