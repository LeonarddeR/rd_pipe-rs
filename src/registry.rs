// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Registry module
// Copyright (C) 2022-2023 Leonard de Ruijter <alderuijter@gmail.com>
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::io;
use tracing::{debug, instrument, trace};
use windows::core::GUID;
use winreg::enums::KEY_ALL_ACCESS;
use winreg::{enums::KEY_WRITE, transaction::Transaction, types::ToRegValue, RegKey, HKEY};

pub const CLSID_RD_PIPE_PLUGIN: GUID = GUID::from_u128(0xD1F74DC79FDE45BE9251FA72D4064DA3);
const RD_PIPE_PLUGIN_NAME: &str = "RdPipe";
pub const COM_CLS_FOLDER: &str = r"SOFTWARE\Classes\CLSID";
const _COM_CLS_CHANNEL_NAMES_VALUE_NAME: &str = "ChannelNames";
const COM_IMPROC_SERVER_FOLDER_NAME: &str = "InprocServer32";
pub const TS_ADD_INS_FOLDER: &str = r"Software\Microsoft\Terminal Server Client\Default\AddIns";
pub const TS_ADD_IN_RD_PIPE_FOLDER_NAME: &str = RD_PIPE_PLUGIN_NAME;
const TS_ADD_IN_NAME_VALUE_NAME: &str = "Name";
const TS_ADD_IN_VIEW_ENABLED_VALUE_NAME: &str = "View Enabled";
#[cfg(target_arch = "x86")]
const CTX_MODULES_FOLDER: &str =
    r"SOFTWARE\Citrix\ICA Client\Engine\Configuration\Advanced\Modules";
#[cfg(target_arch = "x86")]
const CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME: &str = "DvcPlugins";

#[instrument]
pub fn inproc_server_add_to_registry(
    parent_key: HKEY,
    clsid_key: &str,
    dll_path: &str,
    channel_names: &[&str],
) -> io::Result<()> {
    debug!("inproc_server_add_to_registry called");
    let flags = KEY_WRITE;
    trace!("Creating transaction");
    let t = Transaction::new()?;
    let hk = RegKey::predef(parent_key);
    let key_path = format!(r"{}\{{{:?}}}", clsid_key, CLSID_RD_PIPE_PLUGIN);
    trace!("Creating {}", &key_path);
    let (key, _disp) = hk.create_subkey_transacted_with_flags(&key_path, &t, flags)?;
    trace!("Setting default value");
    key.set_value("", &RD_PIPE_PLUGIN_NAME)?;
    trace!("Setting {}", _COM_CLS_CHANNEL_NAMES_VALUE_NAME);
    let channel_names: Vec<&str> = channel_names.into();
    key.set_value(_COM_CLS_CHANNEL_NAMES_VALUE_NAME, &channel_names)?;
    trace!("Creating {}\\{}", &key_path, &COM_IMPROC_SERVER_FOLDER_NAME);
    let (key, _disp) =
        key.create_subkey_transacted_with_flags(COM_IMPROC_SERVER_FOLDER_NAME, &t, flags)?;
    trace!("Setting default value");
    let path_value = dll_path.to_reg_value();
    key.set_raw_value("", &path_value)?;
    trace!("Setting threading model value");
    key.set_value("ThreadingModel", &"Free")?;
    trace!("Committing transaction");
    t.commit()
}

#[instrument]
pub fn delete_from_registry(parent_key: HKEY, reg_path: &str, sub_key: &str) -> io::Result<()> {
    debug!("delete_from_registry called");
    let flags = KEY_ALL_ACCESS;
    let hk = RegKey::predef(parent_key);
    trace!("Opening {}", &reg_path);
    let key = hk.open_subkey_with_flags(reg_path, flags)?;
    trace!("Deleting {}\\{}", &reg_path, &sub_key);
    key.delete_subkey_all(sub_key)
}

#[instrument]
pub fn msts_add_to_registry(parent_key: HKEY) -> io::Result<()> {
    debug!("msts_add_to_registry");
    let flags = KEY_WRITE;
    trace!("Creating transaction");
    let t = Transaction::new()?;
    let hk = RegKey::predef(parent_key);
    let key_path = format!(r"{}\{}", TS_ADD_INS_FOLDER, TS_ADD_IN_RD_PIPE_FOLDER_NAME);
    trace!("Creating {}", &key_path);
    let (key, _disp) = hk.create_subkey_transacted_with_flags(&key_path, &t, flags)?;
    trace!("Setting value {}", TS_ADD_IN_NAME_VALUE_NAME);
    key.set_value(
        TS_ADD_IN_NAME_VALUE_NAME,
        &format!("{{{:?}}}", CLSID_RD_PIPE_PLUGIN),
    )?;
    trace!("Setting value {}", TS_ADD_IN_VIEW_ENABLED_VALUE_NAME);
    key.set_value(TS_ADD_IN_VIEW_ENABLED_VALUE_NAME, &1u32)?;
    trace!("Committing transaction");
    t.commit()
}

#[cfg(target_arch = "x86")]
#[instrument]
pub fn ctx_add_to_registry(parent_key: HKEY) -> io::Result<()> {
    debug!("ctx_add_to_registry called");
    let flags = KEY_READ;
    trace!("Creating transaction");
    let t = Transaction::new()?;
    let hk = RegKey::predef(parent_key);
    trace!("Opening {}", CTX_MODULES_FOLDER);
    let modules_key = hk.open_subkey_transacted_with_flags(CTX_MODULES_FOLDER, &t, flags)?;
    let key_name = format!("DVCPlugin_{}", RD_PIPE_PLUGIN_NAME);
    trace!("Creating {}", &key_name);
    let (key, _disp) = modules_key.create_subkey_transacted_with_flags(key_name, &t, flags)?;
    trace!("Setting value DvcNames");
    key.set_value("DvcNames", &RD_PIPE_PLUGIN_NAME)?;
    trace!("Setting value PluginClassId");
    key.set_value("PluginClassId", &format!("{{{:?}}}", CLSID_RD_PIPE_PLUGIN))?;
    trace!("Opening DVCAdapter key");
    let key = modules_key.open_subkey_transacted_with_flags("DVCAdapter", &t, flags)?;
    let plugins: String = key.get_value(CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME)?;
    trace!("Current plugins under DVC adapter: {}", &plugins);
    let mut plugins_list = plugins.split(',').collect::<Vec<&str>>();
    if !plugins_list.contains(&RD_PIPE_PLUGIN_NAME) {
        debug!("Adding {} to {:?}", &RD_PIPE_PLUGIN_NAME, &plugins_list);
        plugins_list.push(&RD_PIPE_PLUGIN_NAME);
        trace!(
            "Setting value {}",
            CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME
        );
        key.set_value(
            CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME,
            &plugins_list.join(","),
        )?;
    }
    trace!("Committing transaction");
    t.commit()
}

#[cfg(target_arch = "x86")]
#[instrument]
pub fn ctx_delete_from_registry(parent_key: HKEY) -> io::Result<()> {
    debug!("ctx_delete_from_registry called");
    trace!("Creating transaction");
    let t = Transaction::new()?;
    let hk = RegKey::predef(parent_key);
    trace!("Opening {}", CTX_MODULES_FOLDER);
    let modules_key = hk.open_subkey_transacted(CTX_MODULES_FOLDER, &t)?;
    trace!("Opening DVCAdapter key");
    let key = modules_key.open_subkey_transacted_with_flags("DVCAdapter", &t, flags)?;
    let plugins: String = key.get_value(CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME)?;
    trace!("Current plugins under DVC adapter: {}", &plugins);
    let mut plugins_list = plugins.split(',').collect::<Vec<&str>>();
    if plugins_list.contains(&RD_PIPE_PLUGIN_NAME) {
        debug!("removing {} from {:?}", &RD_PIPE_PLUGIN_NAME, &plugins_list);
        plugins_list.retain(|s| s != &RD_PIPE_PLUGIN_NAME);
        trace!(
            "Setting value {}",
            CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME
        );
        key.set_value(
            CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME,
            &plugins_list.join(","),
        )?;
    }
    let key_name = format!("DVCPlugin_{}", RD_PIPE_PLUGIN_NAME);
    trace!("Deleting {}", &key_name);
    modules_key.delete_subkey_transacted_with_flags(key_name, &t, flags)?;
    trace!("Committing transaction");
    t.commit()
}
