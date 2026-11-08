plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// The version comes from Cargo.toml, so it is kept in one place
val cargoVersion = Regex("""(?m)^version = "(\d+)\.(\d+)\.(\d+)"""")
    .find(rootDir.parentFile.resolve("Cargo.toml").readText())!!.groupValues

android {
    namespace = "io.github.wobi848.modbusio"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "io.github.wobi848.modbusio"
        // Android 10: saving to Downloads without storage permission
        minSdk = 29
        targetSdk = 36
        versionCode = cargoVersion[1].toInt() * 10000 + cargoVersion[2].toInt() * 100 + cargoVersion[3].toInt()
        versionName = cargoVersion.drop(1).joinToString(".")
        ndk { abiFilters += "arm64-v8a" }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // modbusio runs as its own process from nativeLibraryDir, so the file must be unpacked
    packaging { jniLibs { useLegacyPackaging = true } }
}

kotlin { jvmToolchain(17) }

// modbusio (Rust) built for Android and shipped as libmodbusio.so: only then may the app run it
val cargoBuild by tasks.registering(Exec::class) {
    val bin = android.ndkDirectory.resolve("toolchains/llvm/prebuilt/linux-x86_64/bin")
    val clang = bin.resolve("aarch64-linux-android29-clang").path
    environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", clang)
    environment("CC_aarch64_linux_android", clang)
    environment("AR_aarch64_linux_android", bin.resolve("llvm-ar").path)
    commandLine(
        rootDir.resolve("build-core.sh").path,
        projectDir.resolve("src/main/jniLibs/arm64-v8a/libmodbusio.so").path,
    )
}
tasks.named("preBuild") { dependsOn(cargoBuild) }
