pub mod Windows {
	pub mod Win32 {
		#[inline]
		pub unsafe fn CancelIoEx(
			hfile: HANDLE,
			lpoverlapped: Option<*const OVERLAPPED>,
		) -> windows_core::BOOL {
			windows_core::link!("kernel32.dll" "system" fn CancelIoEx(hfile : HANDLE, lpoverlapped : *const OVERLAPPED) -> windows_core::BOOL);
			unsafe { CancelIoEx(hfile, lpoverlapped.unwrap_or(core::mem::zeroed()) as _) }
		}
		#[inline]
		pub unsafe fn CloseHandle(hobject: HANDLE) -> windows_core::BOOL {
			windows_core::link!("kernel32.dll" "system" fn CloseHandle(hobject : HANDLE) -> windows_core::BOOL);
			unsafe { CloseHandle(hobject) }
		}
		#[inline]
		pub unsafe fn CoDecrementMTAUsage(cookie: CO_MTA_USAGE_COOKIE) -> windows_core::HRESULT {
			windows_core::link!("ole32.dll" "system" fn CoDecrementMTAUsage(cookie : CO_MTA_USAGE_COOKIE) -> windows_core::HRESULT);
			unsafe { CoDecrementMTAUsage(cookie) }
		}
		#[inline]
		pub unsafe fn CoIncrementMTAUsage() -> windows_core::Result<CO_MTA_USAGE_COOKIE> {
			windows_core::link!("ole32.dll" "system" fn CoIncrementMTAUsage(pcookie : *mut CO_MTA_USAGE_COOKIE) -> windows_core::HRESULT);
			unsafe {
				let mut result__ = core::mem::zeroed();
				CoIncrementMTAUsage(&mut result__).map(|| result__)
			}
		}
		#[inline]
		pub unsafe fn ConnectNamedPipe(
			hnamedpipe: HANDLE,
			lpoverlapped: Option<*mut OVERLAPPED>,
		) -> windows_core::BOOL {
			windows_core::link!("kernel32.dll" "system" fn ConnectNamedPipe(hnamedpipe : HANDLE, lpoverlapped : *mut OVERLAPPED) -> windows_core::BOOL);
			unsafe {
				ConnectNamedPipe(hnamedpipe, lpoverlapped.unwrap_or(core::mem::zeroed()) as _)
			}
		}
		#[inline]
		pub unsafe fn ConvertSidToStringSidW(
			sid: PSID,
			stringsid: *mut windows_core::PWSTR,
		) -> windows_core::BOOL {
			windows_core::link!("advapi32.dll" "system" fn ConvertSidToStringSidW(sid : PSID, stringsid : *mut windows_core::PWSTR) -> windows_core::BOOL);
			unsafe { ConvertSidToStringSidW(sid, stringsid as _) }
		}
		#[inline]
		pub unsafe fn ConvertStringSecurityDescriptorToSecurityDescriptorW<P0>(
			stringsecuritydescriptor: P0,
			stringsdrevision: u32,
			securitydescriptor: *mut PSECURITY_DESCRIPTOR,
			securitydescriptorsize: Option<*mut u32>,
		) -> windows_core::BOOL
		where
			P0: windows_core::Param<windows_core::PCWSTR>,
		{
			windows_core::link!("advapi32.dll" "system" fn ConvertStringSecurityDescriptorToSecurityDescriptorW(stringsecuritydescriptor : windows_core::PCWSTR, stringsdrevision : u32, securitydescriptor : *mut PSECURITY_DESCRIPTOR, securitydescriptorsize : *mut u32) -> windows_core::BOOL);
			unsafe {
				ConvertStringSecurityDescriptorToSecurityDescriptorW(
					stringsecuritydescriptor.param().abi(),
					stringsdrevision,
					securitydescriptor as _,
					securitydescriptorsize.unwrap_or(core::mem::zeroed()) as _,
				)
			}
		}
		#[inline]
		pub unsafe fn CreateEventW<P3>(
			lpeventattributes: Option<*const SECURITY_ATTRIBUTES>,
			bmanualreset: bool,
			binitialstate: bool,
			lpname: P3,
		) -> HANDLE
		where
			P3: windows_core::Param<windows_core::PCWSTR>,
		{
			windows_core::link!("kernel32.dll" "system" fn CreateEventW(lpeventattributes : *const SECURITY_ATTRIBUTES, bmanualreset : windows_core::BOOL, binitialstate : windows_core::BOOL, lpname : windows_core::PCWSTR) -> HANDLE);
			unsafe {
				CreateEventW(
					lpeventattributes.unwrap_or(core::mem::zeroed()) as _,
					bmanualreset.into(),
					binitialstate.into(),
					lpname.param().abi(),
				)
			}
		}
		#[inline]
		pub unsafe fn CreateNamedPipeW<P0>(
			lpname: P0,
			dwopenmode: u32,
			dwpipemode: u32,
			nmaxinstances: u32,
			noutbuffersize: u32,
			ninbuffersize: u32,
			ndefaulttimeout: u32,
			lpsecurityattributes: Option<*const SECURITY_ATTRIBUTES>,
		) -> HANDLE
		where
			P0: windows_core::Param<windows_core::PCWSTR>,
		{
			windows_core::link!("kernel32.dll" "system" fn CreateNamedPipeW(lpname : windows_core::PCWSTR, dwopenmode : u32, dwpipemode : u32, nmaxinstances : u32, noutbuffersize : u32, ninbuffersize : u32, ndefaulttimeout : u32, lpsecurityattributes : *const SECURITY_ATTRIBUTES) -> HANDLE);
			unsafe {
				CreateNamedPipeW(
					lpname.param().abi(),
					dwopenmode,
					dwpipemode,
					nmaxinstances,
					noutbuffersize,
					ninbuffersize,
					ndefaulttimeout,
					lpsecurityattributes.unwrap_or(core::mem::zeroed()) as _,
				)
			}
		}
		#[inline]
		pub unsafe fn DisableThreadLibraryCalls(hlibmodule: HMODULE) -> windows_core::BOOL {
			windows_core::link!("kernel32.dll" "system" fn DisableThreadLibraryCalls(hlibmodule : HMODULE) -> windows_core::BOOL);
			unsafe { DisableThreadLibraryCalls(hlibmodule) }
		}
		#[inline]
		pub unsafe fn DisconnectNamedPipe(hnamedpipe: HANDLE) -> windows_core::BOOL {
			windows_core::link!("kernel32.dll" "system" fn DisconnectNamedPipe(hnamedpipe : HANDLE) -> windows_core::BOOL);
			unsafe { DisconnectNamedPipe(hnamedpipe) }
		}
		#[inline]
		pub unsafe fn GetCurrentProcess() -> HANDLE {
			windows_core::link!("kernel32.dll" "system" fn GetCurrentProcess() -> HANDLE);
			unsafe { GetCurrentProcess() }
		}
		#[inline]
		pub unsafe fn GetModuleFileNameW(
			hmodule: Option<HMODULE>,
			lpfilename: windows_core::PWSTR,
			nsize: u32,
		) -> u32 {
			windows_core::link!("kernel32.dll" "system" fn GetModuleFileNameW(hmodule : HMODULE, lpfilename : windows_core::PWSTR, nsize : u32) -> u32);
			unsafe {
				GetModuleFileNameW(hmodule.unwrap_or(core::mem::zeroed()) as _, lpfilename, nsize)
			}
		}
		#[inline]
		pub unsafe fn GetOverlappedResult(
			hfile: HANDLE,
			lpoverlapped: *const OVERLAPPED,
			lpnumberofbytestransferred: *mut u32,
			bwait: bool,
		) -> windows_core::BOOL {
			windows_core::link!("kernel32.dll" "system" fn GetOverlappedResult(hfile : HANDLE, lpoverlapped : *const OVERLAPPED, lpnumberofbytestransferred : *mut u32, bwait : windows_core::BOOL) -> windows_core::BOOL);
			unsafe {
				GetOverlappedResult(
					hfile,
					lpoverlapped,
					lpnumberofbytestransferred as _,
					bwait.into(),
				)
			}
		}
		#[inline]
		pub unsafe fn GetTokenInformation(
			tokenhandle: HANDLE,
			tokeninformationclass: TOKEN_INFORMATION_CLASS,
			tokeninformation: Option<*mut core::ffi::c_void>,
			tokeninformationlength: u32,
			returnlength: *mut u32,
		) -> windows_core::BOOL {
			windows_core::link!("advapi32.dll" "system" fn GetTokenInformation(tokenhandle : HANDLE, tokeninformationclass : TOKEN_INFORMATION_CLASS, tokeninformation : *mut core::ffi::c_void, tokeninformationlength : u32, returnlength : *mut u32) -> windows_core::BOOL);
			unsafe {
				GetTokenInformation(
					tokenhandle,
					tokeninformationclass,
					tokeninformation.unwrap_or(core::mem::zeroed()) as _,
					tokeninformationlength,
					returnlength as _,
				)
			}
		}
		#[inline]
		pub unsafe fn LocalFree(hmem: HLOCAL) -> HLOCAL {
			windows_core::link!("kernel32.dll" "system" fn LocalFree(hmem : HLOCAL) -> HLOCAL);
			unsafe { LocalFree(hmem) }
		}
		#[inline]
		pub unsafe fn OpenProcessToken(
			processhandle: HANDLE,
			desiredaccess: u32,
			tokenhandle: *mut HANDLE,
		) -> windows_core::BOOL {
			windows_core::link!("advapi32.dll" "system" fn OpenProcessToken(processhandle : HANDLE, desiredaccess : u32, tokenhandle : *mut HANDLE) -> windows_core::BOOL);
			unsafe { OpenProcessToken(processhandle, desiredaccess, tokenhandle as _) }
		}
		#[inline]
		pub unsafe fn ReadFile(
			hfile: HANDLE,
			lpbuffer: Option<*mut core::ffi::c_void>,
			nnumberofbytestoread: u32,
			lpnumberofbytesread: Option<*mut u32>,
			lpoverlapped: Option<*mut OVERLAPPED>,
		) -> windows_core::BOOL {
			windows_core::link!("kernel32.dll" "system" fn ReadFile(hfile : HANDLE, lpbuffer : *mut core::ffi::c_void, nnumberofbytestoread : u32, lpnumberofbytesread : *mut u32, lpoverlapped : *mut OVERLAPPED) -> windows_core::BOOL);
			unsafe {
				ReadFile(
					hfile,
					lpbuffer.unwrap_or(core::mem::zeroed()) as _,
					nnumberofbytestoread,
					lpnumberofbytesread.unwrap_or(core::mem::zeroed()) as _,
					lpoverlapped.unwrap_or(core::mem::zeroed()) as _,
				)
			}
		}
		#[inline]
		pub unsafe fn SetEvent(hevent: HANDLE) -> windows_core::BOOL {
			windows_core::link!("kernel32.dll" "system" fn SetEvent(hevent : HANDLE) -> windows_core::BOOL);
			unsafe { SetEvent(hevent) }
		}
		#[inline]
		pub unsafe fn WaitForMultipleObjects(
			lphandles: &[HANDLE],
			bwaitall: bool,
			dwmilliseconds: u32,
		) -> u32 {
			windows_core::link!("kernel32.dll" "system" fn WaitForMultipleObjects(ncount : u32, lphandles : *const HANDLE, bwaitall : windows_core::BOOL, dwmilliseconds : u32) -> u32);
			unsafe {
				WaitForMultipleObjects(
					lphandles.len().try_into().unwrap(),
					lphandles.as_ptr(),
					bwaitall.into(),
					dwmilliseconds,
				)
			}
		}
		#[inline]
		pub unsafe fn WaitForSingleObject(hhandle: HANDLE, dwmilliseconds: u32) -> u32 {
			windows_core::link!("kernel32.dll" "system" fn WaitForSingleObject(hhandle : HANDLE, dwmilliseconds : u32) -> u32);
			unsafe { WaitForSingleObject(hhandle, dwmilliseconds) }
		}
		#[inline]
		pub unsafe fn WriteFile(
			hfile: HANDLE,
			lpbuffer: Option<*const core::ffi::c_void>,
			nnumberofbytestowrite: u32,
			lpnumberofbyteswritten: Option<*mut u32>,
			lpoverlapped: Option<*mut OVERLAPPED>,
		) -> windows_core::BOOL {
			windows_core::link!("kernel32.dll" "system" fn WriteFile(hfile : HANDLE, lpbuffer : *const core::ffi::c_void, nnumberofbytestowrite : u32, lpnumberofbyteswritten : *mut u32, lpoverlapped : *mut OVERLAPPED) -> windows_core::BOOL);
			unsafe {
				WriteFile(
					hfile,
					lpbuffer.unwrap_or(core::mem::zeroed()) as _,
					nnumberofbytestowrite,
					lpnumberofbyteswritten.unwrap_or(core::mem::zeroed()) as _,
					lpoverlapped.unwrap_or(core::mem::zeroed()) as _,
				)
			}
		}
		pub const CLASS_E_CLASSNOTAVAILABLE: windows_core::HRESULT =
			windows_core::HRESULT(0x80040111_u32 as _);
		pub const CLASS_E_NOAGGREGATION: windows_core::HRESULT =
			windows_core::HRESULT(0x80040110_u32 as _);
		#[repr(transparent)]
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
		pub struct CO_MTA_USAGE_COOKIE(pub *mut core::ffi::c_void);
		pub const DLL_PROCESS_ATTACH: i32 = 1;
		pub const DLL_PROCESS_DETACH: i32 = 0;
		pub const ERROR_BROKEN_PIPE: i32 = 109;
		pub const ERROR_INVALID_PARAMETER: i32 = 87;
		pub const ERROR_IO_PENDING: i32 = 997;
		pub const ERROR_NOT_FOUND: i32 = 1168;
		pub const ERROR_NO_DATA: i32 = 232;
		pub const ERROR_OPERATION_ABORTED: i32 = 995;
		pub const ERROR_PIPE_CONNECTED: i32 = 535;
		pub const ERROR_PIPE_NOT_CONNECTED: i32 = 233;
		pub const E_NOINTERFACE: windows_core::HRESULT = windows_core::HRESULT(0x80004002_u32 as _);
		pub const E_POINTER: windows_core::HRESULT = windows_core::HRESULT(0x80004003_u32 as _);
		pub const E_UNEXPECTED: windows_core::HRESULT = windows_core::HRESULT(0x8000FFFF_u32 as _);
		pub const FILE_FLAG_FIRST_PIPE_INSTANCE: i32 = 524288;
		pub const FILE_FLAG_OVERLAPPED: i32 = 1073741824;
		#[repr(transparent)]
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
		pub struct HANDLE(pub *mut core::ffi::c_void);
		#[repr(transparent)]
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
		pub struct HINSTANCE(pub *mut core::ffi::c_void);
		pub type HLOCAL = HANDLE;
		pub type HMODULE = HINSTANCE;
		windows_core::imp::define_interface!(
			IClassFactory,
			IClassFactory_Vtbl,
			0x00000001_0000_0000_c000_000000000046
		);
		windows_core::imp::interface_hierarchy!(IClassFactory, windows_core::IUnknown);
		impl IClassFactory {
			pub unsafe fn CreateInstance<P0, T>(&self, punkouter: P0) -> windows_core::Result<T>
			where
				P0: windows_core::Param<windows_core::IUnknown>,
				T: windows_core::Interface,
			{
				let mut result__ = core::ptr::null_mut();
				unsafe {
					(windows_core::Interface::vtable(self).CreateInstance)(
						windows_core::Interface::as_raw(self),
						punkouter.param().abi(),
						&T::IID,
						&mut result__,
					)
					.and_then(|| windows_core::imp::Type::from_abi(result__))
				}
			}
			pub unsafe fn LockServer(&self, flock: bool) -> windows_core::HRESULT {
				unsafe {
					(windows_core::Interface::vtable(self).LockServer)(
						windows_core::Interface::as_raw(self),
						flock.into(),
					)
				}
			}
		}
		#[repr(C)]
		pub struct IClassFactory_Vtbl {
			pub base__: windows_core::IUnknown_Vtbl,
			pub CreateInstance: unsafe extern "system" fn(
				*mut core::ffi::c_void,
				*mut core::ffi::c_void,
				*const windows_core::GUID,
				*mut *mut core::ffi::c_void,
			) -> windows_core::HRESULT,
			pub LockServer: unsafe extern "system" fn(
				*mut core::ffi::c_void,
				windows_core::BOOL,
			) -> windows_core::HRESULT,
		}
		pub trait IClassFactory_Impl: windows_core::IUnknownImpl {
			fn CreateInstance(
				&self,
				punkouter: windows_core::Ref<windows_core::IUnknown>,
				riid: *const windows_core::GUID,
				ppvobject: *mut *mut core::ffi::c_void,
			) -> windows_core::Result<()>;
			fn LockServer(&self, flock: windows_core::BOOL) -> windows_core::Result<()>;
		}
		impl IClassFactory_Vtbl {
			pub const fn new<Identity: IClassFactory_Impl, const OFFSET: isize>() -> Self {
				unsafe extern "system" fn CreateInstance<
					Identity: IClassFactory_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
					punkouter: *mut core::ffi::c_void,
					riid: *const windows_core::GUID,
					ppvobject: *mut *mut core::ffi::c_void,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						IClassFactory_Impl::CreateInstance(
							this,
							core::mem::transmute_copy(&punkouter),
							core::mem::transmute_copy(&riid),
							core::mem::transmute_copy(&ppvobject),
						)
						.into()
					}
				}
				unsafe extern "system" fn LockServer<
					Identity: IClassFactory_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
					flock: windows_core::BOOL,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						IClassFactory_Impl::LockServer(this, core::mem::transmute_copy(&flock))
							.into()
					}
				}
				Self {
					base__: windows_core::IUnknown_Vtbl::new::<Identity, OFFSET>(),
					CreateInstance: CreateInstance::<Identity, OFFSET>,
					LockServer: LockServer::<Identity, OFFSET>,
				}
			}
			pub fn matches(iid: &windows_core::GUID) -> bool {
				iid == &<IClassFactory as windows_core::Interface>::IID
			}
		}
		impl windows_core::RuntimeName for IClassFactory {}
		pub const INFINITE: u32 = 4294967295;
		pub const INVALID_HANDLE_VALUE: HANDLE = HANDLE(-1 as _);
		windows_core::imp::define_interface!(
			IPropertyBag,
			IPropertyBag_Vtbl,
			0x55272a00_42cb_11ce_8135_00aa004bb851
		);
		windows_core::imp::interface_hierarchy!(IPropertyBag, windows_core::IUnknown);
		#[repr(C)]
		pub struct IPropertyBag_Vtbl {
			pub base__: windows_core::IUnknown_Vtbl,
			Read: usize,
			Write: usize,
		}
		windows_core::imp::define_interface!(
			IWTSListener,
			IWTSListener_Vtbl,
			0xa1230206_9a39_4d58_8674_cdb4dff4e73b
		);
		windows_core::imp::interface_hierarchy!(IWTSListener, windows_core::IUnknown);
		impl IWTSListener {
			pub unsafe fn GetConfiguration(&self) -> windows_core::Result<IPropertyBag> {
				unsafe {
					let mut result__ = core::mem::zeroed();
					(windows_core::Interface::vtable(self).GetConfiguration)(
						windows_core::Interface::as_raw(self),
						&mut result__,
					)
					.and_then(|| windows_core::imp::Type::from_abi(result__))
				}
			}
		}
		#[repr(C)]
		pub struct IWTSListener_Vtbl {
			pub base__: windows_core::IUnknown_Vtbl,
			pub GetConfiguration: unsafe extern "system" fn(
				*mut core::ffi::c_void,
				*mut *mut core::ffi::c_void,
			) -> windows_core::HRESULT,
		}
		windows_core::imp::define_interface!(
			IWTSListenerCallback,
			IWTSListenerCallback_Vtbl,
			0xa1230203_d6a7_11d8_b9fd_000bdbd1f198
		);
		windows_core::imp::interface_hierarchy!(IWTSListenerCallback, windows_core::IUnknown);
		impl IWTSListenerCallback {
			pub unsafe fn OnNewChannelConnection<P0>(
				&self,
				pchannel: P0,
				data: &windows_core::BSTR,
				pbaccept: *mut windows_core::BOOL,
				ppcallback: *mut Option<IWTSVirtualChannelCallback>,
			) -> windows_core::HRESULT
			where
				P0: windows_core::Param<IWTSVirtualChannel>,
			{
				unsafe {
					(windows_core::Interface::vtable(self).OnNewChannelConnection)(
						windows_core::Interface::as_raw(self),
						pchannel.param().abi(),
						core::mem::transmute_copy(data),
						pbaccept as _,
						core::mem::transmute(ppcallback),
					)
				}
			}
		}
		#[repr(C)]
		pub struct IWTSListenerCallback_Vtbl {
			pub base__: windows_core::IUnknown_Vtbl,
			pub OnNewChannelConnection: unsafe extern "system" fn(
				*mut core::ffi::c_void,
				*mut core::ffi::c_void,
				*mut core::ffi::c_void,
				*mut windows_core::BOOL,
				*mut *mut core::ffi::c_void,
			) -> windows_core::HRESULT,
		}
		pub trait IWTSListenerCallback_Impl: windows_core::IUnknownImpl {
			fn OnNewChannelConnection(
				&self,
				pchannel: windows_core::Ref<IWTSVirtualChannel>,
				data: &windows_core::BSTR,
				pbaccept: *mut windows_core::BOOL,
				ppcallback: windows_core::OutRef<IWTSVirtualChannelCallback>,
			) -> windows_core::Result<()>;
		}
		impl IWTSListenerCallback_Vtbl {
			pub const fn new<Identity: IWTSListenerCallback_Impl, const OFFSET: isize>() -> Self {
				unsafe extern "system" fn OnNewChannelConnection<
					Identity: IWTSListenerCallback_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
					pchannel: *mut core::ffi::c_void,
					data: *mut core::ffi::c_void,
					pbaccept: *mut windows_core::BOOL,
					ppcallback: *mut *mut core::ffi::c_void,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						IWTSListenerCallback_Impl::OnNewChannelConnection(
							this,
							core::mem::transmute_copy(&pchannel),
							core::mem::transmute(&data),
							core::mem::transmute_copy(&pbaccept),
							core::mem::transmute_copy(&ppcallback),
						)
						.into()
					}
				}
				Self {
					base__: windows_core::IUnknown_Vtbl::new::<Identity, OFFSET>(),
					OnNewChannelConnection: OnNewChannelConnection::<Identity, OFFSET>,
				}
			}
			pub fn matches(iid: &windows_core::GUID) -> bool {
				iid == &<IWTSListenerCallback as windows_core::Interface>::IID
			}
		}
		impl windows_core::RuntimeName for IWTSListenerCallback {}
		windows_core::imp::define_interface!(
			IWTSPlugin,
			IWTSPlugin_Vtbl,
			0xa1230201_1439_4e62_a414_190d0ac3d40e
		);
		windows_core::imp::interface_hierarchy!(IWTSPlugin, windows_core::IUnknown);
		impl IWTSPlugin {
			pub unsafe fn Initialize<P0>(&self, pchannelmgr: P0) -> windows_core::HRESULT
			where
				P0: windows_core::Param<IWTSVirtualChannelManager>,
			{
				unsafe {
					(windows_core::Interface::vtable(self).Initialize)(
						windows_core::Interface::as_raw(self),
						pchannelmgr.param().abi(),
					)
				}
			}
			pub unsafe fn Connected(&self) -> windows_core::HRESULT {
				unsafe {
					(windows_core::Interface::vtable(self).Connected)(
						windows_core::Interface::as_raw(self),
					)
				}
			}
			pub unsafe fn Disconnected(&self, dwdisconnectcode: u32) -> windows_core::HRESULT {
				unsafe {
					(windows_core::Interface::vtable(self).Disconnected)(
						windows_core::Interface::as_raw(self),
						dwdisconnectcode,
					)
				}
			}
			pub unsafe fn Terminated(&self) -> windows_core::HRESULT {
				unsafe {
					(windows_core::Interface::vtable(self).Terminated)(
						windows_core::Interface::as_raw(self),
					)
				}
			}
		}
		#[repr(C)]
		pub struct IWTSPlugin_Vtbl {
			pub base__: windows_core::IUnknown_Vtbl,
			pub Initialize: unsafe extern "system" fn(
				*mut core::ffi::c_void,
				*mut core::ffi::c_void,
			) -> windows_core::HRESULT,
			pub Connected:
				unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
			pub Disconnected:
				unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> windows_core::HRESULT,
			pub Terminated:
				unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
		}
		pub trait IWTSPlugin_Impl: windows_core::IUnknownImpl {
			fn Initialize(
				&self,
				pchannelmgr: windows_core::Ref<IWTSVirtualChannelManager>,
			) -> windows_core::Result<()>;
			fn Connected(&self) -> windows_core::Result<()>;
			fn Disconnected(&self, dwdisconnectcode: u32) -> windows_core::Result<()>;
			fn Terminated(&self) -> windows_core::Result<()>;
		}
		impl IWTSPlugin_Vtbl {
			pub const fn new<Identity: IWTSPlugin_Impl, const OFFSET: isize>() -> Self {
				unsafe extern "system" fn Initialize<
					Identity: IWTSPlugin_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
					pchannelmgr: *mut core::ffi::c_void,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						IWTSPlugin_Impl::Initialize(this, core::mem::transmute_copy(&pchannelmgr))
							.into()
					}
				}
				unsafe extern "system" fn Connected<
					Identity: IWTSPlugin_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						IWTSPlugin_Impl::Connected(this).into()
					}
				}
				unsafe extern "system" fn Disconnected<
					Identity: IWTSPlugin_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
					dwdisconnectcode: u32,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						IWTSPlugin_Impl::Disconnected(
							this,
							core::mem::transmute_copy(&dwdisconnectcode),
						)
						.into()
					}
				}
				unsafe extern "system" fn Terminated<
					Identity: IWTSPlugin_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						IWTSPlugin_Impl::Terminated(this).into()
					}
				}
				Self {
					base__: windows_core::IUnknown_Vtbl::new::<Identity, OFFSET>(),
					Initialize: Initialize::<Identity, OFFSET>,
					Connected: Connected::<Identity, OFFSET>,
					Disconnected: Disconnected::<Identity, OFFSET>,
					Terminated: Terminated::<Identity, OFFSET>,
				}
			}
			pub fn matches(iid: &windows_core::GUID) -> bool {
				iid == &<IWTSPlugin as windows_core::Interface>::IID
			}
		}
		impl windows_core::RuntimeName for IWTSPlugin {}
		windows_core::imp::define_interface!(
			IWTSVirtualChannel,
			IWTSVirtualChannel_Vtbl,
			0xa1230207_d6a7_11d8_b9fd_000bdbd1f198
		);
		windows_core::imp::interface_hierarchy!(IWTSVirtualChannel, windows_core::IUnknown);
		impl IWTSVirtualChannel {
			pub unsafe fn Write<P2>(
				&self,
				cbsize: u32,
				pbuffer: *const u8,
				preserved: P2,
			) -> windows_core::HRESULT
			where
				P2: windows_core::Param<windows_core::IUnknown>,
			{
				unsafe {
					(windows_core::Interface::vtable(self).Write)(
						windows_core::Interface::as_raw(self),
						cbsize,
						pbuffer,
						preserved.param().abi(),
					)
				}
			}
			pub unsafe fn Close(&self) -> windows_core::HRESULT {
				unsafe {
					(windows_core::Interface::vtable(self).Close)(windows_core::Interface::as_raw(
						self,
					))
				}
			}
		}
		#[repr(C)]
		pub struct IWTSVirtualChannel_Vtbl {
			pub base__: windows_core::IUnknown_Vtbl,
			pub Write: unsafe extern "system" fn(
				*mut core::ffi::c_void,
				u32,
				*const u8,
				*mut core::ffi::c_void,
			) -> windows_core::HRESULT,
			pub Close: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
		}
		windows_core::imp::define_interface!(
			IWTSVirtualChannelCallback,
			IWTSVirtualChannelCallback_Vtbl,
			0xa1230204_d6a7_11d8_b9fd_000bdbd1f198
		);
		windows_core::imp::interface_hierarchy!(IWTSVirtualChannelCallback, windows_core::IUnknown);
		impl IWTSVirtualChannelCallback {
			pub unsafe fn OnDataReceived(
				&self,
				cbsize: u32,
				pbuffer: *const u8,
			) -> windows_core::HRESULT {
				unsafe {
					(windows_core::Interface::vtable(self).OnDataReceived)(
						windows_core::Interface::as_raw(self),
						cbsize,
						pbuffer,
					)
				}
			}
			pub unsafe fn OnClose(&self) -> windows_core::HRESULT {
				unsafe {
					(windows_core::Interface::vtable(self).OnClose)(
						windows_core::Interface::as_raw(self),
					)
				}
			}
		}
		#[repr(C)]
		pub struct IWTSVirtualChannelCallback_Vtbl {
			pub base__: windows_core::IUnknown_Vtbl,
			pub OnDataReceived: unsafe extern "system" fn(
				*mut core::ffi::c_void,
				u32,
				*const u8,
			) -> windows_core::HRESULT,
			pub OnClose: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
		}
		pub trait IWTSVirtualChannelCallback_Impl: windows_core::IUnknownImpl {
			fn OnDataReceived(&self, cbsize: u32, pbuffer: *const u8) -> windows_core::Result<()>;
			fn OnClose(&self) -> windows_core::Result<()>;
		}
		impl IWTSVirtualChannelCallback_Vtbl {
			pub const fn new<Identity: IWTSVirtualChannelCallback_Impl, const OFFSET: isize>()
			-> Self {
				unsafe extern "system" fn OnDataReceived<
					Identity: IWTSVirtualChannelCallback_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
					cbsize: u32,
					pbuffer: *const u8,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						IWTSVirtualChannelCallback_Impl::OnDataReceived(
							this,
							core::mem::transmute_copy(&cbsize),
							core::mem::transmute_copy(&pbuffer),
						)
						.into()
					}
				}
				unsafe extern "system" fn OnClose<
					Identity: IWTSVirtualChannelCallback_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						IWTSVirtualChannelCallback_Impl::OnClose(this).into()
					}
				}
				Self {
					base__: windows_core::IUnknown_Vtbl::new::<Identity, OFFSET>(),
					OnDataReceived: OnDataReceived::<Identity, OFFSET>,
					OnClose: OnClose::<Identity, OFFSET>,
				}
			}
			pub fn matches(iid: &windows_core::GUID) -> bool {
				iid == &<IWTSVirtualChannelCallback as windows_core::Interface>::IID
			}
		}
		impl windows_core::RuntimeName for IWTSVirtualChannelCallback {}
		windows_core::imp::define_interface!(
			IWTSVirtualChannelManager,
			IWTSVirtualChannelManager_Vtbl,
			0xa1230205_d6a7_11d8_b9fd_000bdbd1f198
		);
		windows_core::imp::interface_hierarchy!(IWTSVirtualChannelManager, windows_core::IUnknown);
		impl IWTSVirtualChannelManager {
			pub unsafe fn CreateListener<P2>(
				&self,
				pszchannelname: *const i8,
				uflags: u32,
				plistenercallback: P2,
			) -> windows_core::Result<IWTSListener>
			where
				P2: windows_core::Param<IWTSListenerCallback>,
			{
				unsafe {
					let mut result__ = core::mem::zeroed();
					(windows_core::Interface::vtable(self).CreateListener)(
						windows_core::Interface::as_raw(self),
						pszchannelname,
						uflags,
						plistenercallback.param().abi(),
						&mut result__,
					)
					.and_then(|| windows_core::imp::Type::from_abi(result__))
				}
			}
		}
		#[repr(C)]
		pub struct IWTSVirtualChannelManager_Vtbl {
			pub base__: windows_core::IUnknown_Vtbl,
			pub CreateListener: unsafe extern "system" fn(
				*mut core::ffi::c_void,
				*const i8,
				u32,
				*mut core::ffi::c_void,
				*mut *mut core::ffi::c_void,
			) -> windows_core::HRESULT,
		}
		#[repr(C)]
		#[derive(Clone, Copy)]
		pub struct OVERLAPPED {
			pub Internal: usize,
			pub InternalHigh: usize,
			pub Anonymous: OVERLAPPED_0,
			pub hEvent: HANDLE,
		}
		impl Default for OVERLAPPED {
			fn default() -> Self {
				unsafe { core::mem::zeroed() }
			}
		}
		#[repr(C)]
		#[derive(Clone, Copy)]
		pub union OVERLAPPED_0 {
			pub Anonymous: OVERLAPPED_0_0,
			pub Pointer: *mut core::ffi::c_void,
		}
		impl Default for OVERLAPPED_0 {
			fn default() -> Self {
				unsafe { core::mem::zeroed() }
			}
		}
		#[repr(C)]
		#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
		pub struct OVERLAPPED_0_0 {
			pub Offset: u32,
			pub OffsetHigh: u32,
		}
		pub const PIPE_ACCESS_DUPLEX: i32 = 3;
		pub const PIPE_READMODE_BYTE: i32 = 0;
		pub const PIPE_TYPE_BYTE: i32 = 0;
		pub const PIPE_WAIT: i32 = 0;
		#[repr(transparent)]
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
		pub struct PSECURITY_DESCRIPTOR(pub *mut core::ffi::c_void);
		#[repr(transparent)]
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
		pub struct PSID(pub *mut core::ffi::c_void);
		pub const SDDL_REVISION_1: i32 = 1;
		#[repr(C)]
		#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
		pub struct SECURITY_ATTRIBUTES {
			pub nLength: u32,
			pub lpSecurityDescriptor: *mut core::ffi::c_void,
			pub bInheritHandle: windows_core::BOOL,
		}
		pub const SE_GROUP_LOGON_ID: u32 = 3221225472;
		#[repr(C)]
		#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
		pub struct SID_AND_ATTRIBUTES {
			pub Sid: PSID,
			pub Attributes: u32,
		}
		pub const S_OK: windows_core::HRESULT = windows_core::HRESULT(0x0_u32 as _);
		#[repr(C)]
		#[derive(Clone, Copy, Debug, Eq, PartialEq)]
		pub struct TOKEN_GROUPS {
			pub GroupCount: u32,
			pub Groups: [SID_AND_ATTRIBUTES; 1],
		}
		impl Default for TOKEN_GROUPS {
			fn default() -> Self {
				unsafe { core::mem::zeroed() }
			}
		}
		pub type TOKEN_INFORMATION_CLASS = i32;
		pub const TOKEN_QUERY: i32 = 8;
		pub const TokenGroups: TOKEN_INFORMATION_CLASS = 2;
		pub const UNICODE_STRING_MAX_CHARS: i32 = 32767;
		pub const WAIT_OBJECT_0: i32 = 0;
		pub const WAIT_TIMEOUT: i32 = 258;
	}
}
