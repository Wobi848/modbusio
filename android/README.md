# modbusio for Android

The Modbus browser on a phone or tablet in the plant's network.

## Install

Copy `app-debug.apk` to the phone, open it and allow installing from this source.

## How it works

- The app ships modbusio itself (Rust, built for `aarch64-linux-android`) as `libmodbusio.so` and
  runs it as a separate process in a foreground service, so recordings keep running with the
  screen off. The UI is the normal web UI in a WebView (`http://127.0.0.1:8082`).
- Modbus TCP only: Android gives a program no serial port. RTU on the phone through a USB serial
  bridge is open (concept section 14, answer 5).
- Exports are saved to Downloads.
- Same pattern as bacnetio for Android: a crash of modbusio does not take the app down, the service
  starts it again. The log is `modbusio.log` in the app data.

## Build

Needs the Android SDK with NDK 28.2 and the Rust target `aarch64-linux-android`
(`rustup target add aarch64-linux-android`).

```sh
cd android
ANDROID_HOME=/usr/lib/android-sdk ./gradlew assembleDebug
```

`build-core.sh` builds the Rust part with the NDK's linker the Gradle task sets.
