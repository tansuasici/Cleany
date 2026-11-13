//! `cleany` CLI. M0 prints version info; `cleany scan <path>` arrives in M1.

fn main() {
    let os = cleany_platform::Os::current();
    println!(
        "cleany {} ({}/{})",
        env!("CARGO_PKG_VERSION"),
        os.as_str(),
        std::env::consts::ARCH
    );
    let _ = cleany_core::Size::default();
}
