// SolidCraft on Android: a NativeActivity around libsolidcraft_android.so (apps/solidcraft-android),
// which `cargo ndk` builds into src/main/jniLibs before the APK is packaged.

plugins {
    id("com.android.application")
}

val repo: File = rootDir.resolve("../../..").canonicalFile
// Comma-separated Android ABIs to build: -Psolidcraft.abis=arm64-v8a,x86_64 (the default covers
// phones and tablets, and the emulator on Apple silicon and x86 machines).
val abis: List<String> = (findProperty("solidcraft.abis") as String? ?: "arm64-v8a,x86_64").split(",").map { it.trim() }
// The Rust profile: the kernel is far too slow unoptimised, so even debug APKs carry a release build.
val cargoProfile: String = findProperty("solidcraft.cargoProfile") as String? ?: "release"

// The version is SolidCraft's own (the workspace Cargo.toml's), so the APK follows upstream.
// A port-only release of the same SolidCraft version raises -Psolidcraft.portRevision (1, 2, …),
// which becomes "-android.N" in the name and the last two digits of the version code.
val solidcraftVersion: String = Regex("""(?m)^version = "(\d+)\.(\d+)\.(\d+)"""")
    .find(repo.resolve("Cargo.toml").readText())?.groupValues?.get(0)?.substringAfter('"')?.removeSuffix("\"")
    ?: error("no version in ${repo.resolve("Cargo.toml")}")
val portRevision: Int = (findProperty("solidcraft.portRevision") as String?)?.toInt() ?: 0
val versionParts: List<Int> = solidcraftVersion.split(".").map { it.toInt() }
val appVersionName: String = if (portRevision > 0) "$solidcraftVersion-android.$portRevision" else solidcraftVersion
// 0.16.2 (revision 0) → 160200: major·10⁶ + minor·10⁴ + patch·10² + revision, always rising.
val appVersionCode: Int = versionParts[0] * 1_000_000 + versionParts[1] * 10_000 + versionParts[2] * 100 + portRevision

base {
    // APKs come out as SolidCraft-<version>-<build type>.apk.
    archivesName = "SolidCraft-$appVersionName"
}

android {
    namespace = "io.solidcraft.android"
    compileSdk = 36
    // The NDK cargo-ndk builds with; AGP strips the library's debug info with its llvm-strip.
    ndkVersion = "29.0.14206865"

    defaultConfig {
        applicationId = "io.solidcraft.android"
        // Vulkan 1.1 (or GLES 3) and AAudio-era NDK APIs.
        minSdk = 26
        targetSdk = 36
        versionCode = appVersionCode
        versionName = appVersionName
        ndk {
            abiFilters += abis
        }
    }

    sourceSets {
        getByName("main") {
            res.directories.add(layout.buildDirectory.dir("generated/icon-res").get().asFile.path)
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // Signed with the debug key until the project has a release key.
            signingConfig = signingConfigs.getByName("debug")
        }
    }
}

// The launcher icon is the app icon's existing export (assets/icons/app), not a copy kept here.
val launcherIcon by tasks.registering(Copy::class) {
    from(repo.resolve("assets/icons/app/linux/exports/hicolor/256x256/apps")) {
        include("*.png")
        rename { "ic_launcher.png" }
    }
    into(layout.buildDirectory.dir("generated/icon-res/mipmap-xxxhdpi"))
}

// cargo from $CARGO_HOME or rustup's default place: Android Studio started from the Dock or a
// launcher doesn't have the shell's PATH.
val cargo: String = listOfNotNull(System.getenv("CARGO_HOME"), System.getProperty("user.home") + "/.cargo")
    .map { File(it, "bin/cargo") }
    .firstOrNull { it.canExecute() }?.path ?: "cargo"

val cargoNdk by tasks.registering(Exec::class) {
    description = "Builds libsolidcraft_android.so for ${abis.joinToString()} with cargo-ndk."
    workingDir = repo
    // cargo-ndk runs `cargo` itself, so it needs cargo's folder on the PATH.
    environment("PATH", File(cargo).parent + File.pathSeparator + System.getenv("PATH"))
    val args = mutableListOf(cargo, "ndk", "--platform", "26", "-o", file("src/main/jniLibs").path)
    abis.forEach { args += listOf("-t", it) }
    args += listOf("build", "-p", "solidcraft-android", "--lib")
    if (cargoProfile == "release") args += "--release" else args += listOf("--profile", cargoProfile)
    commandLine(args)
}

tasks.named("preBuild") {
    dependsOn(launcherIcon, cargoNdk)
}
