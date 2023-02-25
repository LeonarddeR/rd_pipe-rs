use clap::{arg, error::ErrorKind, CommandFactory, Parser, ValueEnum};
use std::{io, path::PathBuf};
use winreg::{
    enums::{
        RegType::REG_EXPAND_SZ, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, KEY_READ,
        KEY_WOW64_32KEY, KEY_WOW64_64KEY, KEY_WRITE,
    },
    transaction::Transaction,
    types::ToRegValue,
    RegKey, HKEY,
};

const CLSID_RD_PIPE_PLUGIN: &str = "{D1F74DC7-9FDE-45BE-9251-FA72D4064DA3}";
const RD_PIPE_PLUGIN_NAME: &str = "RdPipe";
const COM_CLS_FOLDER: &str = r"SOFTWARE\Classes\CLSID";
const COM_CLS_CHANNEL_NAMES_VALUE_NAME: &str = "ChannelNames";
const COM_IMPROC_SERVER_FOLDER_NAME: &str = "InprocServer32";
const TS_ADD_INS_FOLDER: &str = r"Software\Microsoft\Terminal Server Client\Default\AddIns";
const TS_ADD_IN_RD_PIPE_FOLDER_NAME: &str = RD_PIPE_PLUGIN_NAME;
const TS_ADD_IN_NAME_VALUE_NAME: &str = "Name";
const TS_ADD_IN_VIEW_ENABLED_VALUE_NAME: &str = "View Enabled";
const CTX_MODULES_FOLDER: &str =
    r"SOFTWARE\Citrix\ICA Client\Engine\Configuration\Advanced\Modules";
const CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME: &str = "DvcPlugins";

fn inproc_server_add_to_registry(parent_key: HKEY, wow64: bool, path: &str) -> io::Result<()> {
    let flags = KEY_WRITE
        | if wow64 {
            KEY_WOW64_32KEY
        } else {
            KEY_WOW64_64KEY
        };
    let t = Transaction::new()?;
    let hk = RegKey::predef(parent_key);
    let (key, _disp) = hk.create_subkey_transacted_with_flags(
        format!(r"{}\{}", COM_CLS_FOLDER, CLSID_RD_PIPE_PLUGIN),
        &t,
        flags,
    )?;
    key.set_value("", &RD_PIPE_PLUGIN_NAME)?;
    let (key, _disp) =
        key.create_subkey_transacted_with_flags(COM_IMPROC_SERVER_FOLDER_NAME, &t, flags)?;
    let mut path_value = path.to_reg_value();
    if path.to_lowercase().contains("appdata%") {
        path_value.vtype = REG_EXPAND_SZ;
    }
    key.set_raw_value("", &path_value)?;
    key.set_value("ThreadingModel", &"Free")?;
    t.commit()
}

fn delete_from_registry(
    parent_key: HKEY,
    path: &str,
    sub_key: &str,
    wow64: bool,
) -> io::Result<()> {
    let flags = KEY_ALL_ACCESS
        | if wow64 {
            KEY_WOW64_32KEY
        } else {
            KEY_WOW64_64KEY
        };
    let hk = RegKey::predef(parent_key);
    let key = hk.open_subkey_with_flags(path, flags)?;
    key.delete_subkey_all(sub_key)
}

fn msts_add_to_registry(parent_key: HKEY, wow64: bool) -> io::Result<()> {
    let flags = KEY_WRITE
        | if wow64 {
            KEY_WOW64_32KEY
        } else {
            KEY_WOW64_64KEY
        };
    let t = Transaction::new()?;
    let hk = RegKey::predef(parent_key);
    let (key, _disp) = hk.create_subkey_transacted_with_flags(
        format!(r"{}\{}", TS_ADD_INS_FOLDER, TS_ADD_IN_RD_PIPE_FOLDER_NAME),
        &t,
        flags,
    )?;
    key.set_value(TS_ADD_IN_NAME_VALUE_NAME, &CLSID_RD_PIPE_PLUGIN)?;
    key.set_value(TS_ADD_IN_VIEW_ENABLED_VALUE_NAME, &1u32)?;
    t.commit()
}

fn ctx_add_to_registry(parent_key: HKEY) -> io::Result<()> {
    let flags = KEY_READ | KEY_WRITE | KEY_WOW64_32KEY;
    let t = Transaction::new()?;
    let hk = RegKey::predef(parent_key);
    let (modules_key, _disp) =
        hk.create_subkey_transacted_with_flags(CTX_MODULES_FOLDER, &t, flags)?;
    let (key, _disp) = modules_key.create_subkey_transacted_with_flags(
        format!("DVCPlugin_{}", RD_PIPE_PLUGIN_NAME),
        &t,
        flags,
    )?;
    key.set_value("DvcNames", &RD_PIPE_PLUGIN_NAME)?;
    key.set_value("PluginClassId", &CLSID_RD_PIPE_PLUGIN)?;
    let key = modules_key.open_subkey_transacted_with_flags("DVCAdapter", &t, flags)?;
    let plugins: String = key.get_value("DvcPlugins")?;
    let mut plugins_list = plugins.split(',').collect::<Vec<&str>>();
    if !plugins_list.contains(&RD_PIPE_PLUGIN_NAME) {
        plugins_list.push(&RD_PIPE_PLUGIN_NAME);
        key.set_value("DvcPlugins", &plugins_list.join(","))?;
    }
    t.commit()
}

fn ctx_delete_from_registry(parent_key: HKEY) -> io::Result<()> {
    let flags = KEY_READ | KEY_WRITE | KEY_WOW64_32KEY;
    let t = Transaction::new()?;
    let hk = RegKey::predef(parent_key);
    let modules_key = hk.open_subkey_transacted_with_flags(CTX_MODULES_FOLDER, &t, flags)?;
    let key = modules_key.open_subkey_transacted_with_flags("DVCAdapter", &t, flags)?;
    let plugins: String = key.get_value("DvcPlugins")?;
    let mut plugins_list = plugins.split(',').collect::<Vec<&str>>();
    if plugins_list.contains(&RD_PIPE_PLUGIN_NAME) {
        plugins_list.retain(|s| s != &RD_PIPE_PLUGIN_NAME);
        key.set_value("DvcPlugins", &plugins_list.join(","))?;
    }
    modules_key.delete_subkey_transacted_with_flags(
        format!("DVCPlugin_{}", RD_PIPE_PLUGIN_NAME),
        &t,
        flags,
    )?;
    t.commit()
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Scope {
    CurrentUser,
    LocalMachine,
}

#[derive(Parser, Debug)]
#[command(author, version)]
struct Cli {
    #[arg(value_enum)]
    action: Action,
    #[arg(long)]
    com_server: bool,
    #[arg(short = 'p', long, value_name = "DLL_PATH", required_if_eq_all = [("com_server", "true"), ("action", "register")])]
    dll_path: Option<PathBuf>,
    #[arg(long)]
    rdp: bool,
    #[arg(long, requires = "wow64")]
    citrix: bool,
    #[arg(short, long, default_value_t = Scope::CurrentUser)]
    #[arg(value_enum)]
    scope: Scope,
    #[arg(short, long)]
    wow64: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Action {
    Register,
    Unregister,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    if let Some(p) = &cli.dll_path {
        if !p.exists() {
            let mut cmd = Cli::command();
            cmd.error(
                ErrorKind::InvalidValue,
                format!("The file '{}' does not exist", p.display()),
            )
            .exit();
        }
    }
    let scope_hkey = match cli.scope {
        Scope::CurrentUser => HKEY_CURRENT_USER,
        Scope::LocalMachine => HKEY_LOCAL_MACHINE,
    };
    match cli.action {
        Action::Register => {
            if cli.com_server {
                inproc_server_add_to_registry(
                    scope_hkey,
                    cli.wow64,
                    cli.dll_path.unwrap().to_str().unwrap(),
                )?;
            }
            if cli.rdp {
                msts_add_to_registry(scope_hkey, cli.wow64)?;
            }
            if cli.citrix {
                ctx_add_to_registry(scope_hkey)?;
            }
        }
        Action::Unregister => {
            if cli.com_server {
                delete_from_registry(scope_hkey, COM_CLS_FOLDER, CLSID_RD_PIPE_PLUGIN, cli.wow64)?;
            }
            if cli.rdp {
                delete_from_registry(
                    scope_hkey,
                    TS_ADD_INS_FOLDER,
                    TS_ADD_IN_RD_PIPE_FOLDER_NAME,
                    cli.wow64,
                )?;
            }
            if cli.citrix {
                ctx_delete_from_registry(scope_hkey)?;
            }
        }
    }
    Ok(())
}
