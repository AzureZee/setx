use std::{
    env::{self, set_current_dir},
    io,
};

fn main() -> io::Result<()> {
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut profile = env::var("PROFILE").unwrap();
    if let Ok(target) = env::var("TARGET") {
        profile = format!("{target}/{profile}");
    }
    let out = format!("{root}/target/{profile}");
    // let out = Path::new(&out);
    set_current_dir(out)?;
    dbg!(env::current_dir()?);
    let original = "setv.exe";
    let link = "setm.exe";

    #[cfg(windows)]
    {
        dbg!("windows");
        std::os::windows::fs::symlink_file(original, link)?;
    }
    #[cfg(unix)]
    {
        dbg!("unix");
        std::os::unix::fs::symlink(original, link)?;
    }
    println!("cargo::rerun-if-changed=build.rs");
    Ok(())
}
