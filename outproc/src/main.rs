// RD Pipe: Windows Remote Desktop Services Dynamic Virtual Channel implementation using named pipes, written in Rust
// Main entrypoint for outproc binary crate
// Copyright (C) 2022 Leonard de Ruijter <alderuijter@gmail.com>
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

#![windows_subsystem = "windows"]
use std::env::temp_dir;

use rd_pipe_core::class_factory::{ClassFactory, IID_I_RD_PIPE_PLUGIN};
use tracing::instrument;
use windows::Win32::System::Com::IClassFactory;
use windows::Win32::System::Com::{
    CoRegisterClassObject, CoRevokeClassObject, CLSCTX_LOCAL_SERVER, REGCLS_MULTIPLEUSE,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[instrument]
fn main() {
    let file_appender = tracing_appender::rolling::never(temp_dir(), "RdPipe.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop)
        .unwrap();
    let factory: IClassFactory = ClassFactory.into();
    let res = unsafe {
        CoRegisterClassObject(
            &IID_I_RD_PIPE_PLUGIN,
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
