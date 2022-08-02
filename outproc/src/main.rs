#![windows_subsystem = "windows"]
use dvc_core::class_factory::{ClassFactory, IID_I_DVC_PLUGIN};


use windows::Win32::System::Com::{
    CoRegisterClassObject, CoRevokeClassObject, CLSCTX_LOCAL_SERVER,
    REGCLS_MULTIPLEUSE,
};
use windows::Win32::System::Com::{IClassFactory};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};


fn main() {
    //unsafe { CoInitializeEx(core::ptr::null_mut(), COINIT_MULTITHREADED) }.unwrap();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop)
        .unwrap();
    let factory: IClassFactory = ClassFactory.into();
    let res = unsafe {
        CoRegisterClassObject(
            &IID_I_DVC_PLUGIN,
            &factory,
            CLSCTX_LOCAL_SERVER,
            REGCLS_MULTIPLEUSE,
        )
    };
    let cookie: u32 = res.unwrap();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => {
                unsafe { CoRevokeClassObject(cookie) }.unwrap();
                *control_flow = ControlFlow::Exit;
            }
            _ => (),
        }
    });
}
