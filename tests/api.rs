//! The web API without a browser, against the simulated devices: the router is called directly.

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use modbusio::server::{AppState, router};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn call(
    app: &axum::Router,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::HOST, "127.0.0.1:8082");
    let body = match body {
        Some(body) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(request.body(body).expect("request"))
        .await
        .expect("answer");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024 * 1024)
        .await
        .expect("body");
    let value =
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!(String::from_utf8_lossy(&bytes)));
    (status, value)
}

fn app() -> axum::Router {
    router(Arc::new(AppState::new(None).expect("state")))
}

async fn drive_on_site(app: &axum::Router) -> String {
    let address = common::gateway().await;
    let key = format!("tcp:{address}/2");
    let (status, _) = call(
        app,
        "POST",
        "/api/devices/add",
        Some(json!({ "key": key, "name": "Drive" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    key
}

fn drive_profile() -> Value {
    json!({
        "id": "", "name": "FU-7", "points": [
            { "name": "Set frequency", "area": "holding_register", "address": 1, "type": "u16", "factor": 0.01, "unit": "Hz", "writable": true, "min": 0, "max": 50 },
            { "name": "Output frequency", "area": "holding_register", "address": 100, "type": "u16", "factor": 0.01, "unit": "Hz" },
            { "name": "Run", "area": "coil", "address": 0, "type": "bool", "writable": true },
            { "name": "Ready", "area": "discrete_input", "address": 0, "type": "bool" }
        ]
    })
}

#[tokio::test]
async fn a_device_added_by_hand_is_asked_whether_it_answers() {
    let app = app();
    let address = common::gateway().await;
    let add = |key: String| json!({ "key": key });
    let (status, site) = call(
        &app,
        "POST",
        "/api/devices/add",
        Some(add(format!("tcp:{address}/2"))),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{site}");
    assert_eq!(site["check"], json!({ "answered": true, "error": null }));
    assert_eq!(site["devices"].as_array().map(Vec::len), Some(1));
    // behind the gateway unit 9 is not reached: the gateway says so, the device is kept anyway
    let (_, site) = call(
        &app,
        "POST",
        "/api/devices/add",
        Some(add(format!("tcp:{address}/9"))),
    )
    .await;
    assert_eq!(
        site["check"],
        json!({ "answered": false, "error": "exception_0b" })
    );
    let (_, site) = call(
        &app,
        "POST",
        "/api/devices/add",
        Some(add("tcp:127.0.0.1:1/1".into())),
    )
    .await;
    assert_eq!(site["check"]["error"], "open");
    // a serial line is not asked: another master may be active on it
    let (_, site) = call(
        &app,
        "POST",
        "/api/devices/add",
        Some(add(format!("rtu-tcp:{address}/3"))),
    )
    .await;
    assert_eq!(site["check"], Value::Null);
    assert_eq!(site["devices"].as_array().map(Vec::len), Some(4));
}

#[tokio::test]
async fn info_names_the_program() {
    let (status, info) = call(&app(), "GET", "/api/info", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(info["program"], "modbusio");
}

#[tokio::test]
async fn foreign_pages_are_refused() {
    let state = AppState {
        web_address: Some("127.0.0.1:8082".parse().expect("address")),
        ..AppState::new(None).expect("state")
    };
    let app = router(Arc::new(state));
    let request = Request::builder()
        .uri("/api/info")
        .header(header::HOST, "127.0.0.1:8082")
        .header(header::ORIGIN, "http://evil.example")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("answer");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_device_with_a_profile_is_read() {
    let app = app();
    let key = drive_on_site(&app).await;
    let (status, read) = call(
        &app,
        "POST",
        "/api/devices/read",
        Some(json!({ "key": key })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "no profile yet: {read}");
    let (status, profile) = call(&app, "POST", "/api/profiles", Some(drive_profile())).await;
    assert_eq!(status, StatusCode::OK, "{profile}");
    assert_eq!(profile["id"], "fu-7");
    let (status, _) = call(
        &app,
        "POST",
        "/api/devices/update",
        Some(json!({ "key": key, "profile": "fu-7" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, read) = call(
        &app,
        "POST",
        "/api/devices/read",
        Some(json!({ "key": key })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{read}");
    let texts: Vec<&str> = read["points"]
        .as_array()
        .expect("points")
        .iter()
        .map(|point| point["reading"]["text"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(texts, ["25 Hz", "24.98 Hz", "0", "1"]);
    assert_eq!(read["failed"], 0);
}

#[tokio::test]
async fn writing_needs_the_unlock_and_can_be_put_back() {
    let app = app();
    let key = drive_on_site(&app).await;
    call(&app, "POST", "/api/profiles", Some(drive_profile())).await;
    call(
        &app,
        "POST",
        "/api/devices/update",
        Some(json!({ "key": key, "profile": "fu-7" })),
    )
    .await;
    let write = json!({ "key": key, "point": "Set frequency", "value": "30" });
    let (status, refused) = call(&app, "POST", "/api/write", Some(write.clone())).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    call(
        &app,
        "POST",
        "/api/session",
        Some(json!({ "unlocked": true })),
    )
    .await;
    let (status, _) = call(
        &app,
        "POST",
        "/api/write",
        Some(json!({ "key": key, "point": "Set frequency", "value": "60" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "above the point's max");
    // checking a value, as the confirmation is asked, writes nothing and logs nothing
    let check = |value: &str| json!({ "key": key, "point": "Set frequency", "value": value, "check": true });
    let (status, refused) = call(&app, "POST", "/api/write", Some(check("abc"))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let (status, checked) = call(&app, "POST", "/api/write", Some(check("30"))).await;
    assert_eq!(status, StatusCode::OK, "{checked}");
    assert_eq!(checked, json!({ "checked": true, "text": "30 Hz" }));
    let (_, journal) = call(&app, "GET", "/api/journal", None).await;
    assert_eq!(journal["entries"], json!([]), "a check is not logged");
    let (status, entry) = call(&app, "POST", "/api/write", Some(write)).await;
    assert_eq!(status, StatusCode::OK, "{entry}");
    assert_eq!(entry["before"], json!([2500]));
    assert_eq!(entry["after"], json!([3000]));
    assert_eq!(entry["before_text"], "25 Hz");
    // a second write to the same place: putting back still means the value before the first
    let again = json!({ "key": key, "point": "Set frequency", "value": "35" });
    let (status, entry) = call(&app, "POST", "/api/write", Some(again)).await;
    assert_eq!(status, StatusCode::OK, "{entry}");
    assert_eq!(entry["before_text"], "30 Hz");
    let (status, coil) = call(
        &app,
        "POST",
        "/api/write",
        Some(json!({ "key": key, "point": "Run", "value": "on" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{coil}");
    let (_, journal) = call(&app, "GET", "/api/journal", None).await;
    assert_eq!(journal["standing"], 2);
    let standing = &journal["entries"][1];
    assert_eq!(standing["standing"], true, "{standing}");
    assert_eq!(standing["original"], json!([2500]));
    assert_eq!(standing["original_text"], "25 Hz");
    let (status, restored) = call(&app, "POST", "/api/restore", Some(json!({ "all": true }))).await;
    assert_eq!(status, StatusCode::OK, "{restored}");
    let put_back = &restored["results"][0]["entry"];
    assert_eq!(put_back["written"], json!([2500]), "{restored}");
    assert_eq!(put_back["text"], "25 Hz", "the words of the value written");
    let (_, journal) = call(&app, "GET", "/api/journal", None).await;
    assert_eq!(journal["standing"], 0, "{journal}");
    let (_, read) = call(
        &app,
        "POST",
        "/api/devices/read",
        Some(json!({ "key": key, "points": ["Set frequency", "Run"] })),
    )
    .await;
    assert_eq!(read["points"][0]["reading"]["text"], "25 Hz");
    assert_eq!(read["points"][1]["reading"]["text"], "0");
}

#[tokio::test]
async fn a_raw_write_needs_its_own_confirmation() {
    let app = app();
    let key = drive_on_site(&app).await;
    call(
        &app,
        "POST",
        "/api/session",
        Some(json!({ "unlocked": true })),
    )
    .await;
    let raw = json!({ "area": "holding_register", "address": 2, "type": "u16" });
    let (status, _) = call(
        &app,
        "POST",
        "/api/write",
        Some(json!({ "key": key, "raw": raw, "value": "250" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, entry) = call(
        &app,
        "POST",
        "/api/write",
        Some(json!({ "key": key, "raw": raw, "value": "250", "confirm_raw": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{entry}");
    assert_eq!(entry["written"], json!([250]));
    let (_, refused) = call(&app, "POST", "/api/write", Some(json!({ "key": key, "raw": { "area": "holding_register", "address": 100, "type": "u16" }, "value": "1", "confirm_raw": true }))).await;
    assert!(
        refused["error"].is_string(),
        "a read-only register refuses: {refused}"
    );
}

#[tokio::test]
async fn nothing_is_written_to_unit_0_the_broadcast() {
    let app = app();
    let address = common::meter(false).await;
    let key = format!("tcp:{address}/0");
    call(
        &app,
        "POST",
        "/api/devices/add",
        Some(json!({ "key": key })),
    )
    .await;
    call(
        &app,
        "POST",
        "/api/session",
        Some(json!({ "unlocked": true })),
    )
    .await;
    let raw = json!({ "area": "holding_register", "address": 100, "type": "u16" });
    let (status, answer) = call(
        &app,
        "POST",
        "/api/write",
        Some(json!({ "key": key, "raw": raw, "value": "60", "confirm_raw": true })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    assert!(
        answer["error"]
            .as_str()
            .expect("error")
            .contains("broadcast")
    );
}

#[tokio::test]
async fn the_explorer_finds_the_addresses_and_the_limit() {
    let app = app();
    let key = drive_on_site(&app).await;
    let (status, explored) = call(
        &app,
        "POST",
        "/api/explore",
        Some(json!({ "key": key, "area": "holding_register", "start": 0, "count": 120 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(explored["found"], 8);
    assert!(
        explored["limits"]["most_registers"]
            .as_u64()
            .expect("limit")
            <= 16
    );
    let (status, _) = call(
        &app,
        "POST",
        "/api/explore",
        Some(json!({ "key": key, "area": "holding_register", "start": 0, "count": 5000 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_imported_list_is_previewed_with_values_from_the_device() {
    let app = app();
    let address = common::gateway().await;
    let key = format!("tcp:{address}/1");
    call(
        &app,
        "POST",
        "/api/devices/add",
        Some(json!({ "key": key })),
    )
    .await;
    let list = "Register;Name;Type;Unit\n40001;Energy;float;kWh\n40003;Flow temperature;float;°C\n40009;Volume;uint16;m³\n";
    let encoded = base64(list.as_bytes());
    let (status, preview) = call(&app, "POST", "/api/profiles/import", Some(json!({ "file_name": "hm50.csv", "content_base64": encoded, "key": key, "order": "CDAB" }))).await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["imported"]["convention"], "reference");
    assert_eq!(
        preview["imported"]["points"]
            .as_array()
            .expect("points")
            .len(),
        3
    );
    assert_eq!(preview["preview"][0]["text"], "1234.5 kWh");
}

#[tokio::test]
async fn a_search_of_the_units_behind_a_gateway_adds_them_to_the_site() {
    let app = app();
    let address = common::gateway().await;
    let (status, _) = call(
        &app,
        "POST",
        "/api/scan/units",
        Some(
            json!({ "endpoint": format!("tcp:{address}"), "from": 1, "to": 6, "timeout_ms": 300 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &app,
        "POST",
        "/api/scan/units",
        Some(json!({ "endpoint": format!("tcp:{address}") })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "one search at a time");
    // what it finds joins the open site, so no other site is opened meanwhile
    let other = json!({ "name": "Other plant" });
    let (status, refused) = call(&app, "POST", "/api/sites/new", Some(other)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    let mut report = json!({});
    for _ in 0..100 {
        report = call(&app, "GET", "/api/scan", None).await.1;
        if report["finished"] == true {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let keys: Vec<&str> = report["devices"]
        .as_array()
        .expect("devices")
        .iter()
        .filter_map(|device| device["key"].as_str())
        .collect();
    assert_eq!(keys.len(), 3, "{report}");
    let (_, site) = call(&app, "GET", "/api/site", None).await;
    assert_eq!(site["devices"].as_array().expect("devices").len(), 3);
}

#[tokio::test]
async fn wrong_requests_are_refused_in_words() {
    let app = app();
    for (path, body) in [
        (
            "/api/scan/units",
            json!({ "endpoint": "tcp:10.0.0.1", "from": 0, "to": 5 }),
        ),
        (
            "/api/scan/units",
            json!({ "endpoint": "serial:COM3:fast:8E1" }),
        ),
        ("/api/scan/tcp", json!({ "subnet": "10.0.0.0/8" })),
        ("/api/devices/add", json!({ "key": "10.0.0.1" })),
        (
            "/api/profiles",
            json!({ "id": "", "name": "", "points": [] }),
        ),
        (
            "/api/profiles",
            json!({ "id": "", "name": "X", "points": [{ "name": "a", "area": "coil", "address": 0, "type": "f32" }] }),
        ),
    ] {
        let (status, answer) = call(&app, "POST", path, Some(body.clone())).await;
        assert!(status.is_client_error(), "{path} {body}: {status}");
        assert!(
            answer["error"].is_string() || answer.is_string(),
            "{path}: {answer}"
        );
    }
}

#[tokio::test]
async fn a_site_file_is_checked_whole_before_anything_of_it_is_kept() {
    let folder = std::env::temp_dir().join(format!("modbusio-api-site-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let data = folder.join("data");
    let app = router(Arc::new(AppState::new(Some(data.clone())).expect("state")));
    let device = json!({ "key": "tcp:10.0.0.5:502/1", "name": "Drive", "profile": "fu-7" });
    let file = |profile_id: &str| {
        let mut profile = drive_profile();
        profile["id"] = json!(profile_id);
        json!({
            "modbusio_site": 1,
            "site": { "id": "plant", "name": "Plant", "devices": [device.clone(), device.clone()] },
            "profiles": [profile],
        })
    };

    // a profile's id names its file: one that leads out of the data folder is refused
    let (status, refused) = call(&app, "POST", "/api/site.json", Some(file("../../escaped"))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(!folder.join("escaped.json").exists());
    let (_, sites) = call(&app, "GET", "/api/sites", None).await;
    assert_eq!(sites.as_array().map(Vec::len), Some(1), "no site was added");

    let (status, site) = call(&app, "POST", "/api/site.json", Some(file("fu-7"))).await;
    assert_eq!(status, StatusCode::OK, "{site}");
    assert_eq!(site["name"], "Plant");
    let devices = site["devices"].as_array().expect("devices");
    assert_eq!(devices.len(), 1, "a device named twice is there once");
    assert_eq!(devices[0]["profile_name"], "FU-7");
    assert!(data.join("profiles/fu-7.json").exists());

    let (status, exported) = call(&app, "GET", "/api/site.json", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(exported["modbusio_site"], 1);
    assert_eq!(exported["site"]["devices"][0]["key"], "tcp:10.0.0.5:502/1");
    assert_eq!(exported["profiles"][0]["id"], "fu-7");
    let _ = std::fs::remove_dir_all(folder);
}

#[tokio::test]
async fn writes_over_one_another_are_put_back_newest_first() {
    let app = app();
    let key = drive_on_site(&app).await;
    call(
        &app,
        "POST",
        "/api/session",
        Some(json!({ "unlocked": true })),
    )
    .await;
    // the two ramps, 300 each: written as one 32-bit value, then the second register alone
    let raw = |address: u16, kind: &str, value: &str| {
        let place = json!({ "area": "holding_register", "address": address, "type": kind });
        json!({ "key": key, "raw": place, "confirm_raw": true, "value": value })
    };
    let (status, both) = call(&app, "POST", "/api/write", Some(raw(2, "u32", "458761"))).await;
    assert_eq!(
        (status, &both["after"]),
        (StatusCode::OK, &json!([7, 9])),
        "{both}"
    );
    let (status, second) = call(&app, "POST", "/api/write", Some(raw(3, "u16", "50"))).await;
    assert_eq!(
        (status, &second["before"]),
        (StatusCode::OK, &json!([9])),
        "{second}"
    );
    let (status, restored) = call(&app, "POST", "/api/restore", Some(json!({ "all": true }))).await;
    assert_eq!(status, StatusCode::OK, "{restored}");
    let results = restored["results"].as_array().expect("results");
    assert_eq!(
        (&results[0]["id"], &results[1]["id"]),
        (&json!(2), &json!(1))
    );
    assert_eq!(
        results[1]["entry"]["after"],
        json!([300, 300]),
        "{restored}"
    );
    let ramps = json!({ "key": key, "area": "holding_register", "start": 2, "count": 2 });
    let (_, explored) = call(&app, "POST", "/api/explore", Some(ramps)).await;
    let held: Vec<&Value> = explored["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| &row["value"])
        .collect();
    assert_eq!(held, [&json!(300), &json!(300)], "{explored}");
}

#[tokio::test]
async fn a_write_from_the_command_line_beside_the_server_keeps_its_number() {
    let data = std::env::temp_dir().join(format!("modbusio-api-log-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let app = router(Arc::new(AppState::new(Some(data.clone())).expect("state")));
    let key = drive_on_site(&app).await;
    call(
        &app,
        "POST",
        "/api/session",
        Some(json!({ "unlocked": true })),
    )
    .await;
    let raw = json!({ "area": "holding_register", "address": 1, "type": "u16" });
    let write =
        |value: &str| json!({ "key": key, "raw": raw, "confirm_raw": true, "value": value });
    let (status, first) = call(&app, "POST", "/api/write", Some(write("3000"))).await;
    assert_eq!(
        (status, &first["id"]),
        (StatusCode::OK, &json!(1)),
        "{first}"
    );

    // the command line, started with the same data folder, logs a write of its own
    let log = data.join(modbusio::storage::JOURNAL);
    let (stored, _) = modbusio::storage::load_entries(&log).expect("the log file");
    let mut beside = stored[0].clone();
    beside.id = 2;
    beside.address = 7;
    modbusio::storage::append_entry(&log, &beside).expect("the log file");

    let (_, journal) = call(&app, "GET", "/api/journal", None).await;
    assert_eq!(journal["entries"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        journal["standing"], 2,
        "its write stands as well: {journal}"
    );
    let (status, next) = call(&app, "POST", "/api/write", Some(write("3500"))).await;
    assert_eq!((status, &next["id"]), (StatusCode::OK, &json!(3)), "{next}");
    let _ = std::fs::remove_dir_all(data);
}

/// Samples of the one recorded point.
async fn samples(app: &axum::Router) -> usize {
    let (_, recording) = call(app, "GET", "/api/recording", None).await;
    recording["series"][0]["samples"]
        .as_array()
        .map_or(0, Vec::len)
}

// The clock is the test's: nothing here waits for a device. The port does not exist, so every
// round records that reading failed.
#[tokio::test(start_paused = true)]
async fn stopping_and_starting_again_leaves_one_sampling() {
    let app = app();
    let key = "serial:/dev/modbusio-no-such-port:9600:8N1/1";
    call(
        &app,
        "POST",
        "/api/devices/add",
        Some(json!({ "key": key })),
    )
    .await;
    call(&app, "POST", "/api/profiles", Some(drive_profile())).await;
    let update = json!({ "key": key, "profile": "fu-7" });
    call(&app, "POST", "/api/devices/update", Some(update)).await;
    let point = json!({ "key": key, "point": "Set frequency" });
    let (status, added) = call(&app, "POST", "/api/recording/add", Some(point)).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    let start = json!({ "interval": 5 });

    call(&app, "POST", "/api/recording/start", Some(start.clone())).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(samples(&app).await, 1);
    // stopped and started while the first sampling sleeps until its next round
    call(&app, "POST", "/api/recording/stop", None).await;
    call(&app, "POST", "/api/recording/start", Some(start)).await;
    tokio::time::sleep(Duration::from_secs(11)).await;
    assert_eq!(
        samples(&app).await,
        4,
        "one from the first start, then one every 5 s from the second"
    );

    call(&app, "POST", "/api/recording/stop", None).await;
    tokio::time::sleep(Duration::from_secs(11)).await;
    assert_eq!(samples(&app).await, 4, "stopped");
}

fn base64(bytes: &[u8]) -> String {
    const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::new();
    for chunk in bytes.chunks(3) {
        let bits = chunk
            .iter()
            .fold(0u32, |bits, &byte| (bits << 8) | u32::from(byte))
            << (8 * (3 - chunk.len()));
        for index in 0..=chunk.len() {
            text.push(LETTERS[((bits >> (18 - 6 * index)) & 63) as usize] as char);
        }
    }
    while !text.len().is_multiple_of(4) {
        text.push('=');
    }
    text
}
