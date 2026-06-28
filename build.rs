use std::env;
use std::path::Path;

fn main() {
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut profile = env::var("PROFILE").unwrap();
    if cfg!(unix) {
        let target = env::var("TARGET").unwrap();
        profile = format!("{target}/{profile}");
    }
    let out = format!("{root}/target/{profile}");

    env::set_current_dir(out).unwrap();
    dbg!(env::current_dir().unwrap());

    let link = Path::new("setm.exe");
    if link.is_symlink() {
        return;
    }
    let original = "setv.exe";

    #[cfg(windows)]
    {
        dbg!("windows");
        std::os::windows::fs::symlink_file(original, link).unwrap();
    }
    #[cfg(unix)]
    {
        dbg!("unix");
        std::os::unix::fs::symlink(original, link).unwrap();
    }
    println!("cargo::rerun-if-changed=build.rs");
}
