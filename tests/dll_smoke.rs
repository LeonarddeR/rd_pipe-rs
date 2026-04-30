// Copyright (C) 2026 Leonard de Ruijter
// Integration tests that verify the DLL loads and exposes its exports.

mod common;

use libloading::Library;

#[test]
fn dll_loads_and_exports_present() {
    let path = common::dll_path();
    assert!(
        path.is_file(),
        "rd_pipe.dll not found at {} — run `cargo test` (which builds the cdylib) instead of `cargo run`",
        path.display()
    );

    // Library::new on Windows calls LoadLibraryW + DllMain(DLL_PROCESS_ATTACH).
    // SAFETY: we control the DLL we are loading; it is the artifact this very
    // test crate produced. Required-export resolution below confirms identity.
    let lib = unsafe { Library::new(&path) }.expect("LoadLibrary failed");

    // Resolve the three exports the COM in-proc server contract requires.
    // Each `lib.get` is unsafe because the function-pointer signature is
    // declared by the caller; we only verify presence here, not call them.
    unsafe {
        let _: libloading::Symbol<unsafe extern "system" fn()> =
            lib.get(b"DllMain\0").expect("DllMain export missing");
        let _: libloading::Symbol<unsafe extern "system" fn()> =
            lib.get(b"DllGetClassObject\0").expect("DllGetClassObject export missing");
        let _: libloading::Symbol<unsafe extern "system" fn()> =
            lib.get(b"DllInstall\0").expect("DllInstall export missing");
    }

    // Library drops here -> FreeLibrary -> DllMain(DLL_PROCESS_DETACH).
}
