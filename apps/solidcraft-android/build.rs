//! The Android library's link: std's pthread key calls go to src/tls_keys.rs, whose keys do not
//! run out as Android's do.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        for call in ["pthread_getspecific", "pthread_key_create", "pthread_key_delete", "pthread_setspecific"] {
            println!("cargo:rustc-cdylib-link-arg=-Wl,--wrap={call}");
        }
    }
}
