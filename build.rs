use std::env;
use std::path::{Path, PathBuf};

fn make_relative(from: &Path, to: &Path) -> PathBuf {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();

    // 找公共前缀长度
    let mut common = 0;
    while common < from.len() && common < to.len() && from[common] == to[common] {
        common += 1;
    }

    let mut rel_path = PathBuf::new();
    // from 中剩余的每一层都加一个 ".."
    for _ in common..from.len() {
        rel_path.push("..");
    }
    // 再拼上 to 中剩余的部分
    for comp in &to[common..] {
        rel_path.push(comp.as_os_str());
    }
    rel_path
}

fn main() {
    let install_root = if let Ok(var) = env::var("CARGO_HOME") {
        Path::new(&var).join("bin")
    } else {
        Path::new("~/.cargo/bin").to_path_buf()
    };

    env::set_current_dir(&install_root).unwrap();
    dbg!(&install_root);

    let setm = Path::new("setm.exe");
    let link = Path::new("setv.exe");

    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut original = make_relative(&install_root, Path::new(&root));

    original.push("target");
    if cfg!(unix) {
        let target = env::var("TARGET").unwrap();
        original.push(target);
    }
    let profile = env::var("PROFILE").unwrap();
    original.push(profile);
    original.push("setv.exe");
    dbg!(&original);

    if link.is_symlink() {
        return;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_file;
        dbg!("windows");
        symlink_file(original, link).unwrap();
        symlink_file(link, setm).unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        dbg!("unix");
        symlink(original, link).unwrap();
        symlink(link, setm).unwrap();
    }
    println!("cargo::rerun-if-changed=build.rs");
}
