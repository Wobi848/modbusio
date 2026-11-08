//! Finding devices (concept section 5): a TCP subnet, units on a serial line, the line's settings,
//! and listening to a line without sending.

mod common;

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use common::sim;
use modbusio::diagnosis::Note;
use modbusio::scan::{self, Search};

#[tokio::test]
async fn a_subnet_search_finds_a_direct_device_once_and_names_a_gateway() {
    let meter = tokio::net::TcpListener::bind("127.0.0.2:0")
        .await
        .expect("a free port");
    let port = meter.local_addr().expect("bound").port();
    let Ok(gateway) = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 3], port))).await
    else {
        eprintln!("port {port} is taken on 127.0.0.3; skipped");
        return;
    };
    tokio::spawn(sim::serve_tcp(
        meter,
        sim::Units::new(vec![(1, sim::energy_meter())]).answering_any_unit(),
        false,
    ));
    tokio::spawn(sim::serve_tcp(
        gateway,
        common::gateway_units().behind_gateway(Duration::from_millis(30)),
        false,
    ));
    let search = Arc::new(Search::default());
    let hosts = (2..=5).map(|last| Ipv4Addr::new(127, 0, 0, last)).collect();
    scan::scan_tcp(
        Arc::new(common::links()),
        hosts,
        port,
        Duration::from_millis(300),
        Duration::from_millis(500),
        search.clone(),
    )
    .await;
    let report = search.report.lock().expect("report");
    let keys: Vec<&str> = report
        .devices
        .iter()
        .map(|device| device.key.as_str())
        .collect();
    assert_eq!(
        keys,
        [
            format!("tcp:127.0.0.2:{port}/1"),
            format!("tcp:127.0.0.3:{port}/1")
        ],
        "{report:?}"
    );
    assert_eq!(report.devices[0].product.as_deref(), Some("EM-340"));
    let notes: Vec<&Note> = report.notes.iter().map(|note| &note.note).collect();
    assert!(notes.contains(&&Note::AnswersEveryUnit {
        address: format!("127.0.0.2:{port}")
    }));
    assert!(notes.contains(&&Note::Gateway {
        address: format!("127.0.0.3:{port}")
    }));
    assert_eq!(report.hosts.len(), 2);
}

#[cfg(unix)]
#[tokio::test]
async fn units_and_settings_are_found_on_a_serial_line() {
    let port = common::trunk();
    let links = Arc::new(common::links());
    let search = Arc::new(Search::default());
    let found = scan::find_settings(
        links.clone(),
        port.clone(),
        vec![1, 2],
        Duration::from_millis(200),
        search.clone(),
    )
    .await;
    let settings = found.expect("a pseudo-terminal answers at the first setting tried");
    assert_eq!(
        (settings.baud, settings.frame_text()),
        (9600, "8E1".to_string())
    );
    let search = Arc::new(Search::default());
    let endpoint = modbusio::endpoint::Endpoint::Serial(settings);
    scan::search_units(
        links,
        endpoint,
        1,
        8,
        Duration::from_millis(150),
        0,
        search.clone(),
    )
    .await;
    let units: Vec<u8> = search
        .report
        .lock()
        .expect("report")
        .devices
        .iter()
        .map(|device| device.target.unit)
        .collect();
    assert_eq!(units, [1, 2, 5]);
}

#[cfg(unix)]
#[tokio::test]
async fn listening_hears_another_master_without_sending() {
    use tokio::io::AsyncWriteExt;
    use tokio_serial::SerialPort;
    let (mut master, line) = tokio_serial::SerialStream::pair().expect("a pseudo-terminal");
    let name = line.name().expect("a name");
    std::mem::forget(line);
    let request = modbusio::protocol::rtu::frame(5, &[0x03, 0x00, 0x00, 0x00, 0x02]);
    let answer = modbusio::protocol::rtu::frame(5, &[0x03, 0x04, 0x00, 0x01, 0x00, 0x02]);
    tokio::spawn(async move {
        for _ in 0..40 {
            let _ = master.write_all(&request).await;
            let _ = master.write_all(&answer).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });
    let listened = modbusio::listen::listen(&name, Duration::from_millis(300))
        .await
        .expect("listened");
    assert_eq!(listened.active.as_deref(), Some("9600 8E1"));
    assert!(listened.trials[0].heard.units.contains(&5));
    assert!(!listened.quiet);
}
