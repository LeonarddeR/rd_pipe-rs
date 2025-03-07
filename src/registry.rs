// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Registry module
// Copyright (C) 2022-2025 Leonard de Ruijter <alderuijter@gmail.com>
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

use tracing::{debug, instrument, trace};
use windows::core::GUID;
use windows_registry::{Key, Transaction};

pub const CLSID_RD_PIPE_PLUGIN: GUID = GUID::from_u128(0xD1F74DC79FDE45BE9251FA72D4064DA3);
const RD_PIPE_PLUGIN_NAME: &str = "RdPipe";
pub const COM_CLS_FOLDER: &str = r"SOFTWARE\Classes\CLSID";
const _COM_CLS_CHANNEL_NAMES_VALUE_NAME: &str = "ChannelNames";
const COM_IMPROC_SERVER_FOLDER_NAME: &str = "InprocServer32";
pub const TS_ADD_INS_FOLDER: &str = r"Software\Microsoft\Terminal Server Client\Default\AddIns";
pub const TS_ADD_IN_RD_PIPE_FOLDER_NAME: &str = RD_PIPE_PLUGIN_NAME;
const TS_ADD_IN_NAME_VALUE_NAME: &str = "Name";
const TS_ADD_IN_VIEW_ENABLED_VALUE_NAME: &str = "View Enabled";
const CTX_MODULES_FOLDER: &str =
    r"SOFTWARE\Citrix\ICA Client\Engine\Configuration\Advanced\Modules";
const CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME: &str = "DvcPlugins";

#[instrument]
pub fn inproc_server_add_to_registry(
    parent_key: &Key,
    clsid_key: &str,
    dll_path: &str,
    channel_names: &[&str],
) -> windows_core::Result<()> {
    debug!("inproc_server_add_to_registry called");
    trace!("Creating transaction");
    let t = Transaction::new()?;
    let key_path = format!(r"{}\{{{:?}}}", clsid_key, CLSID_RD_PIPE_PLUGIN);
    trace!("Creating {}", &key_path);
    let key = parent_key
        .options()
        .write()
        .create()
        .transaction(&t)
        .open(&key_path)?;
    trace!("Setting default value");
    key.set_string("", RD_PIPE_PLUGIN_NAME)?;
    trace!("Setting {}", _COM_CLS_CHANNEL_NAMES_VALUE_NAME);
    let channel_names: Vec<&str> = channel_names.into();
    key.set_multi_string(_COM_CLS_CHANNEL_NAMES_VALUE_NAME, &channel_names)?;
    trace!("Creating {}\\{}", &key_path, &COM_IMPROC_SERVER_FOLDER_NAME);
    let key = key.open(COM_IMPROC_SERVER_FOLDER_NAME)?;
    trace!("Setting default value");
    key.set_string("", dll_path)?;
    trace!("Setting threading model value");
    key.set_string("ThreadingModel", "Free")?;
    trace!("Committing transaction");
    t.commit()
}

#[instrument]
pub fn delete_from_registry(
    parent_key: &Key,
    reg_path: &str,
    sub_key: &str,
) -> windows_core::Result<()> {
    debug!("delete_from_registry called");
    trace!("Opening {}", &reg_path);
    let key = parent_key.open(reg_path)?;
    trace!("Deleting {}\\{}", &reg_path, &sub_key);
    key.remove_tree(sub_key)
}

#[instrument]
pub fn msts_add_to_registry(parent_key: &Key) -> windows_core::Result<()> {
    debug!("msts_add_to_registry");
    trace!("Creating transaction");
    let t = Transaction::new()?;
    let key_path = format!(r"{}\{}", TS_ADD_INS_FOLDER, TS_ADD_IN_RD_PIPE_FOLDER_NAME);
    trace!("Creating {}", &key_path);
    let key = parent_key
        .options()
        .write()
        .create()
        .transaction(&t)
        .open(&key_path)?;
    trace!("Setting value {}", TS_ADD_IN_NAME_VALUE_NAME);
    key.set_string(
        TS_ADD_IN_NAME_VALUE_NAME,
        format!("{{{:?}}}", CLSID_RD_PIPE_PLUGIN),
    )?;
    trace!("Setting value {}", TS_ADD_IN_VIEW_ENABLED_VALUE_NAME);
    key.set_u32(TS_ADD_IN_VIEW_ENABLED_VALUE_NAME, 1)?;
    trace!("Committing transaction");
    t.commit()
}

#[instrument]
pub fn ctx_add_to_registry(parent_key: &Key) -> windows_core::Result<()> {
    debug!("ctx_add_to_registry called");
    trace!("Creating transaction");
    let t = Transaction::new()?;
    trace!("Opening {}", CTX_MODULES_FOLDER);
    let modules_key = parent_key
        .options()
        .read()
        .write()
        .create()
        .transaction(&t)
        .open(CTX_MODULES_FOLDER)?;
    let key_name = format!("DVCPlugin_{}", RD_PIPE_PLUGIN_NAME);
    trace!("Creating {}", &key_name);
    let key = modules_key.open(key_name)?;
    trace!("Setting value DvcNames");
    key.set_string("DvcNames", RD_PIPE_PLUGIN_NAME)?;
    trace!("Setting value PluginClassId");
    key.set_string("PluginClassId", format!("{{{:?}}}", CLSID_RD_PIPE_PLUGIN))?;
    trace!("Opening DVCAdapter key");
    let key = modules_key.open("DVCAdapter")?;
    let plugins: String = key.get_string(CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME)?;
    trace!("Current plugins under DVC adapter: {}", &plugins);
    let mut plugins_list = plugins.split(',').collect::<Vec<&str>>();
    if !plugins_list.contains(&RD_PIPE_PLUGIN_NAME) {
        debug!("Adding {} to {:?}", &RD_PIPE_PLUGIN_NAME, &plugins_list);
        plugins_list.push(RD_PIPE_PLUGIN_NAME);
        trace!(
            "Setting value {}",
            CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME
        );
        key.set_string(
            CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME,
            plugins_list.join(","),
        )?;
    }
    trace!("Committing transaction");
    t.commit()
}

#[instrument]
pub fn ctx_delete_from_registry(parent_key: &Key) -> windows_core::Result<()> {
    debug!("ctx_delete_from_registry called");
    trace!("Creating transaction");
    let t = Transaction::new()?;
    trace!("Opening {}", CTX_MODULES_FOLDER);
    let modules_key = parent_key
        .options()
        .read()
        .write()
        .transaction(&t)
        .open(CTX_MODULES_FOLDER)?;
    trace!("Opening DVCAdapter key");
    let key = modules_key.open("DVCAdapter")?;
    let plugins = key.get_string(CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME)?;
    trace!("Current plugins under DVC adapter: {}", &plugins);
    let mut plugins_list = plugins.split(',').collect::<Vec<&str>>();
    if plugins_list.contains(&RD_PIPE_PLUGIN_NAME) {
        debug!("removing {} from {:?}", &RD_PIPE_PLUGIN_NAME, &plugins_list);
        plugins_list.retain(|s| s != &RD_PIPE_PLUGIN_NAME);
        trace!(
            "Setting value {}",
            CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME
        );
        key.set_string(
            CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME,
            plugins_list.join(","),
        )?;
    }
    let key_name = format!("DVCPlugin_{}", RD_PIPE_PLUGIN_NAME);
    trace!("Deleting {}", &key_name);
    modules_key.remove_tree(key_name)?;
    trace!("Committing transaction");
    t.commit()
}
