use std::{
    env,
    fs::{File, read_to_string, remove_file},
    io::{self, Write},
    process::{Command, exit},
};

use windows_registry::{CURRENT_USER, Key};

type IoResult<T> = Result<T, io::Error>;

fn show_help(code: i32) {
    let msg = "Usage:
    setv <var-name> [value]    if value is none, will remove this var.
    setv -[(a|append)|(p|prepend)|(d|delete)] <paths...>
    setv -[e|edit-path] <editor>    use editor edit PATH";

    eprintln!("{}", msg);
    exit(code)
}

fn main() -> IoResult<()> {
    let mut args = env::args().skip(1).peekable();
    match (&args.next(), args.peek()) {
        (Some(flag), ..)
            if flag.starts_with("-") && matches!(flag.trim_start_matches("-"), "h" | "help") =>
        {
            show_help(0)
        }
        (Some(flag), Some(_)) if flag.starts_with("-") => {
            let cu_env = CURRENT_USER.options().read().write().open(ENVIRONMENT)?;
            let args: Vec<_> = args.collect();
            let flag = flag.trim_start_matches("-");
            set_path(args, flag, cu_env)?;
        }
        (Some(name), value) if !name.starts_with("-") => {
            if let Some(value) = value {
                set_var(name, value)?
            } else {
                remove_var(name)?
            }
        }
        _ => show_help(1),
    }
    Ok(())
}

fn set_path(args: Vec<String>, flag: &str, cu_env: Key) -> IoResult<()> {
    let mut path_var: String = cu_env.get_string(PATH)?;

    const SEMICOLON: &str = ";";
    const LF: &str = "\n";

    macro_rules! set_path_val {
        ($val:tt) => {
            cu_env.set_expand_string(PATH, &$val)?;
        };
    }
    match flag {
        "a" | "append" => {
            let path_args = args.join(SEMICOLON);
            if !path_var.ends_with(SEMICOLON) {
                path_var.push_str(SEMICOLON);
            }
            path_var.push_str(&path_args);
            set_path_val!(path_var);
        }
        "p" | "prepend" => {
            let mut path_args = args.join(SEMICOLON);
            path_args.push_str(SEMICOLON);
            path_args.push_str(&path_var);
            set_path_val!(path_args);
        }
        "d" | "delete" => {
            let new = path_var
                .split(SEMICOLON)
                .filter(|p| !p.is_empty())
                .filter_map(|p| (!args.iter().any(|a| a == p)).then_some(p))
                .collect::<Vec<_>>()
                .join(SEMICOLON);
            set_path_val!(new);
        }
        "e" | "edit-path" => {
            let tmp_file_path = env::temp_dir().join("edit-path_xxxxxx.txt");
            if tmp_file_path.exists() {
                remove_file(&tmp_file_path)?;
            }
            let mut file = File::create_new(&tmp_file_path)?;
            let buf = path_var.replace(SEMICOLON, LF);
            file.write_all(buf.as_bytes())?;

            let editor: &str = &args[0];
            let mut cmd = Command::new("cmd.exe");
            cmd.args(["/c", editor]);
            if ["code", "zed"].contains(&editor) {
                cmd.arg("--wait");
            }
            cmd.arg(&tmp_file_path).spawn()?.wait()?;

            let new = read_to_string(&tmp_file_path)?;
            let new = new.trim_end().replace(LF, SEMICOLON);
            set_path_val!(new);

            remove_file(tmp_file_path)?;
        }
        _ => {
            show_help(1);
        }
    }
    Ok(())
}

fn set_var(name: &str, value: &str) -> IoResult<()> {
    let key = CURRENT_USER.options().write().open(ENVIRONMENT)?;
    key.set_string(name, value)?;
    Ok(())
}
fn remove_var(name: &str) -> IoResult<()> {
    let key = CURRENT_USER.options().write().open(ENVIRONMENT)?;
    key.remove_value(name)?;
    Ok(())
}
const ENVIRONMENT: &str = "Environment";
const PATH: &str = "Path";
