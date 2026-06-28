use std::{
    env,
    fs::{File, read_to_string, remove_file},
    io::{self, Write},
    path::Path,
    process::{Command, exit},
};

use windows::Win32::{
    Foundation::{HANDLE, LPARAM, WPARAM},
    Security::{GetTokenInformation, TOKEN_ELEVATION, TokenElevation},
    UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    },
};
use windows_registry::{CURRENT_USER, Key, LOCAL_MACHINE, ValueIterator};

type IoResult<T> = Result<T, io::Error>;

fn main() -> IoResult<()> {
    let mut args = env::args().skip(1).peekable();
    match (args.next().as_deref(), args.peek()) {
        (Some("-h") | Some("-help"), ..) => show_help(0),
        (Some("-l") | Some("-list"), ..) => list_env()?,
        (Some(flag), Some(_)) if flag.starts_with("-") => {
            let key = env_scope()?.read_write()?;
            let args: Vec<_> = args.collect();
            let flag = flag.trim_start_matches("-");
            set_path_var(key, flag, args)?;
        }
        (Some(name), value) if !name.starts_with("-") => {
            set_env_var(env_scope()?.write()?, name, value.map(|x| x.as_str()))?;
        }
        _ => show_help(1),
    }
    Ok(())
}

fn set_path_var(key: Key, flag: &str, args: Vec<String>) -> IoResult<()> {
    let mut path_var: String = key.get_string(PATH)?;

    const SEMICOLON: &str = ";";
    const LF: &str = "\n";

    let new_path = match flag {
        "a" | "append" => {
            let path_args = args.join(SEMICOLON);
            if !path_var.ends_with(SEMICOLON) {
                path_var.push_str(SEMICOLON);
            }
            path_var.push_str(&path_args);
            path_var
        }
        "p" | "prepend" => {
            let mut new = args.join(SEMICOLON);
            new.push_str(SEMICOLON);
            new.push_str(&path_var);
            new
        }
        "d" | "delete" => {
            let new = path_var
                .split(SEMICOLON)
                .filter(|p| !p.is_empty())
                .filter_map(|p| (!args.iter().any(|a| a == p)).then_some(p))
                .collect::<Vec<_>>()
                .join(SEMICOLON);
            new
        }
        "e" | "edit-path" => {
            use std::time::SystemTime;

            let time = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                Ok(n) => n.as_secs(),
                Err(_) => panic!("SystemTime before UNIX EPOCH!"),
            };
            let path = format!("edit-path_{}.txt",time);
            let tmp_file_path = env::temp_dir().join(path);
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
            remove_file(tmp_file_path)?;
            new
        }
        _ => {
            show_help(1);
        }
    };
    set_env_var(key, PATH, Some(new_path.as_str()))?;
    Ok(())
}

const USER_ENV_PATH: &str = "Environment";
const MACHINE_ENV_PATH: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
const PATH: &str = "Path";

fn list_env() -> IoResult<()> {
    let machine = MACHINE_ENV.read()?;
    let user = USER_ENV.read()?;
    let user_values = user.values()?;
    let machine_values = machine.values()?;

    let format_section = |title: &str, values: ValueIterator<'_>| -> String {
        let body = values
            .map(|(n, v)| format!("{n} = {}", String::try_from(v).unwrap_or_default()))
            .collect::<Vec<_>>()
            .join("\n");
        format!("========== {title} ==========\n{body}")
    };

    print!(
        "{}\n\n{}",
        format_section("User Environment Variables", user_values),
        format_section("Machine Environment Variables", machine_values),
    );
    Ok(())
}

/// if arg0 is 'setm' and current process is elevated,
/// return MACHINE_ENV, else USER_ENV.
fn env_scope() -> IoResult<Env> {
    let arg0 = env::args_os().next().unwrap();
    let arg0 = Path::new(&arg0).file_stem().unwrap();
    let env = if arg0 == "setm" {
        if !is_elevated()? {
            eprintln!("[Error]: permission denied!");
            show_help(1)
        }
        MACHINE_ENV
    } else {
        USER_ENV
    };
    Ok(env)
}

struct Env {
    scope: &'static Key,
    path: &'static str,
}

impl Env {
    fn read(&self) -> IoResult<Key> {
        let key = self.scope.options().read().open(self.path)?;
        Ok(key)
    }
    fn write(&self) -> IoResult<Key> {
        let key = self.scope.options().write().open(self.path)?;
        Ok(key)
    }
    fn read_write(&self) -> IoResult<Key> {
        let key = self.scope.options().read().write().open(self.path)?;
        Ok(key)
    }
}
const MACHINE_ENV: Env = Env {
    scope: LOCAL_MACHINE,
    path: MACHINE_ENV_PATH,
};
const USER_ENV: Env = Env {
    scope: CURRENT_USER,
    path: USER_ENV_PATH,
};

/// 设置环境变量.
/// 如果 `value` 为 `None`, 就移除参数 `name` 所代表的环境变量
fn set_env_var(key: Key, name: &str, value: Option<&str>) -> IoResult<()> {
    match value {
        Some(value) => {
            if value.contains("%") || name == PATH {
                key.set_expand_string(name, value)?;
            } else {
                key.set_string(name, value)?;
            }
        }
        None => {
            key.remove_value(name)?;
        }
    }
    notify_environment_changed();
    Ok(())
}

/// 检查当前进程是否已提权
fn is_elevated() -> windows::core::Result<bool> {
    // https://github.com/microsoft/windows-rs/issues/1363#issuecomment-1018671172
    const CURRENT_PROCESS_TOKEN: HANDLE = HANDLE(-4isize as *mut _);

    // https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation
    unsafe {
        let mut elevation = TOKEN_ELEVATION::default();
        let tokeninformation = Some(&mut elevation as *mut _ as *mut _);
        let tokeninformationlength = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        let mut ret_len = 0u32;

        GetTokenInformation(
            CURRENT_PROCESS_TOKEN,
            TokenElevation,
            tokeninformation,
            tokeninformationlength,
            &mut ret_len,
        )?;
        Ok(elevation.TokenIsElevated != 0)
    }
}

/// 通知所有顶级窗口环境变量已变更
fn notify_environment_changed() {
    // https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-settingchange
    // To effect a change in the environment variables for the system or the user,
    // broadcast this message with lParam set to the string "Environment".
    const ENV_W: windows::core::PCWSTR = windows::core::w!("Environment");
    let lparam = LPARAM(ENV_W.as_ptr() as isize);

    let timeout = 1000; // 1s
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            lparam,
            SMTO_ABORTIFHUNG,
            timeout,
            None,
        );
    }
}

fn show_help(code: i32) -> ! {
    let msg = "Usage:
    By default, 'setv' modify user environment variables;
    to modify machine environment variables,
    please run as administrator, and use command 'setm' instead.

    setv <var-name> [value]    if value is none, will remove this var.
    setv -[(a|append)|(p|prepend)|(d|delete)] <paths...>
    setv -[e|edit-path] <editor>    use editor edit PATH";

    eprintln!("{}", msg);
    exit(code)
}
