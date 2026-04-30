// Copyright (C) 2026 Leonard de Ruijter
// End-to-end integration tests for the rd_pipe COM plugin.

mod common;

use serial_test::serial;

#[test]
#[serial]
fn factory_creates_plugin() {
    let _hkcu = common::HkcuOverride::new().expect("override hkcu");
    let dll = common::DllHandle::load();
    let plugin = common::create_plugin(&dll);
    // create_plugin succeeds => DllGetClassObject + CreateInstance(IWTSPlugin) both worked.
    drop(plugin);
}
