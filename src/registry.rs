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

use clap::Parser;
use clap::{arg, ValueEnum};
use std::io;
use windows::core::GUID;
use winreg::{
    enums::{RegType::REG_EXPAND_SZ, KEY_ALL_ACCESS, KEY_READ, KEY_WRITE},
    transaction::Transaction,
    types::ToRegValue,
    RegKey, HKEY,
};

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

pub fn inproc_server_add_to_registry(parent_key: HKEY, path: &str) -> io::Result<()> {
    let flags = KEY_WRITE;
    let t = Transaction::new()?;
    let hk = RegKey::predef(parent_key);
    let (key, _disp) = hk.create_subkey_transacted_with_flags(
        format!(r"{}\{{{:?}}}", COM_CLS_FOLDER, CLSID_RD_PIPE_PLUGIN),
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

pub fn delete_from_registry(parent_key: HKEY, reg_path: &str, sub_key: &str) -> io::Result<()> {
    let flags = KEY_ALL_ACCESS;
    let hk = RegKey::predef(parent_key);
    let key = hk.open_subkey_with_flags(reg_path, flags)?;
    key.delete_subkey_all(sub_key)
}

pub fn msts_add_to_registry(parent_key: HKEY) -> io::Result<()> {
    let flags = KEY_WRITE;
    let t = Transaction::new()?;
    let hk = RegKey::predef(parent_key);
    let (key, _disp) = hk.create_subkey_transacted_with_flags(
        format!(r"{}\{}", TS_ADD_INS_FOLDER, TS_ADD_IN_RD_PIPE_FOLDER_NAME),
        &t,
        flags,
    )?;
    key.set_value(
        TS_ADD_IN_NAME_VALUE_NAME,
        &format!("{{{:?}}}", CLSID_RD_PIPE_PLUGIN),
    )?;
    key.set_value(TS_ADD_IN_VIEW_ENABLED_VALUE_NAME, &1u32)?;
    t.commit()
}

pub fn ctx_add_to_registry(parent_key: HKEY) -> io::Result<()> {
    let flags = KEY_READ;
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
    key.set_value("PluginClassId", &format!("{{{:?}}}", CLSID_RD_PIPE_PLUGIN))?;
    let key = modules_key.open_subkey_transacted_with_flags("DVCAdapter", &t, flags)?;
    let plugins: String = key.get_value(CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME)?;
    let mut plugins_list = plugins.split(',').collect::<Vec<&str>>();
    if !plugins_list.contains(&RD_PIPE_PLUGIN_NAME) {
        plugins_list.push(&RD_PIPE_PLUGIN_NAME);
        key.set_value(
            CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME,
            &plugins_list.join(","),
        )?;
    }
    t.commit()
}

pub fn ctx_delete_from_registry(parent_key: HKEY) -> io::Result<()> {
    let flags = KEY_READ;
    let t = Transaction::new()?;
    let hk = RegKey::predef(parent_key);
    let modules_key = hk.open_subkey_transacted_with_flags(CTX_MODULES_FOLDER, &t, flags)?;
    let key = modules_key.open_subkey_transacted_with_flags("DVCAdapter", &t, flags)?;
    let plugins: String = key.get_value(CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME)?;
    let mut plugins_list = plugins.split(',').collect::<Vec<&str>>();
    if plugins_list.contains(&RD_PIPE_PLUGIN_NAME) {
        plugins_list.retain(|s| s != &RD_PIPE_PLUGIN_NAME);
        key.set_value(
            CTX_MODULE_DVC_ADAPTER_PLUGINS_VALUE_NAAME,
            &plugins_list.join(","),
        )?;
    }
    modules_key.delete_subkey_transacted_with_flags(
        format!("DVCPlugin_{}", RD_PIPE_PLUGIN_NAME),
        &t,
        flags,
    )?;
    t.commit()
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Scope {
    CurrentUser,
    LocalMachine,
}

#[derive(Parser, Debug)]
#[command(author, version)]
pub struct Cli {
    #[arg(short)]
    pub com_server: bool,
    #[arg(short)]
    pub rdp: bool,
    #[arg(short = 'x')]
    pub citrix: bool,
    #[arg(short, default_value_t = Scope::CurrentUser)]
    #[arg(value_enum)]
    pub scope: Scope,
}
