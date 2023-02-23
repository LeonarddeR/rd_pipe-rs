use core::fmt;
use std::io;

use winreg::{transaction::Transaction, RegKey, HKEY};

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

#[derive(Debug, Clone, Copy)]
enum Architecture {
    X86,
    AMD64,
    ARM64,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Architecture::X86 => write!(f, "x86"),
            Architecture::AMD64 => write!(f, "amd64"),
            Architecture::ARM64 => write!(f, "arm64"),
        }
    }
}

fn inproc_server_add_to_registry(
    parent_key: HKEY,
    architecture: Architecture,
    path: &str,
) -> io::Result<()> {
    let t = Transaction::new()?;
    let hk = RegKey::predef(parent_key);
    let (key, _disp) =
        hk.create_subkey_transacted(format!(r"{}\{}", COM_CLS_FOLDER, CLSID_RD_PIPE_PLUGIN), &t)?;
    key.set_value("", &RD_PIPE_PLUGIN_NAME)?;
    let (key, _disp) = key.create_subkey_transacted(COM_IMPROC_SERVER_FOLDER_NAME, &t)?;
    key.set_value("", &path)?;
    key.set_value("ThreadingModel", &"Free")?;
    t.commit()
}

fn inproc_server_delete_from_registry(
    parentKey: HKEY,
    architecture: Architecture,
) -> io::Result<()> {
    let hk = RegKey::predef(parentKey);
    hk.delete_subkey_all(format!(r"{}\{}", COM_CLS_FOLDER, CLSID_RD_PIPE_PLUGIN))
}

fn main() {
    println!("Hello, world!");
}
