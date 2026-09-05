pub mod Windows {
	pub mod Win32 {
		#[inline]
		pub unsafe fn RegLoadAppKeyW<P0>(
			lpfile: P0,
			phkresult: *mut HKEY,
			samdesired: REGSAM,
			dwoptions: u32,
			reserved: Option<u32>,
		) -> LSTATUS
		where
			P0: windows_core::Param<windows_core::PCWSTR>,
		{
			windows_core::link!("advapi32.dll" "system" fn RegLoadAppKeyW(lpfile : windows_core::PCWSTR, phkresult : *mut HKEY, samdesired : REGSAM, dwoptions : u32, reserved : u32) -> LSTATUS);
			unsafe {
				RegLoadAppKeyW(
					lpfile.param().abi(),
					phkresult as _,
					samdesired,
					dwoptions,
					reserved.unwrap_or(core::mem::zeroed()) as _,
				)
			}
		}
		#[inline]
		pub unsafe fn RegOverridePredefKey(hkey: HKEY, hnewhkey: Option<HKEY>) -> LSTATUS {
			windows_core::link!("advapi32.dll" "system" fn RegOverridePredefKey(hkey : HKEY, hnewhkey : HKEY) -> LSTATUS);
			unsafe { RegOverridePredefKey(hkey, hnewhkey.unwrap_or(core::mem::zeroed()) as _) }
		}
		#[repr(transparent)]
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
		pub struct ACCESS_MASK(pub u32);
		pub const CLASS_E_CLASSNOTAVAILABLE: windows_core::HRESULT =
			windows_core::HRESULT(0x80040111_u32 as _);
		#[repr(C)]
		#[derive(Clone, Copy)]
		pub union CY {
			pub Anonymous: CY_0,
			pub int64: i64,
		}
		impl Default for CY {
			fn default() -> Self {
				unsafe { core::mem::zeroed() }
			}
		}
		#[repr(C)]
		#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
		pub struct CY_0 {
			pub Lo: u32,
			pub Hi: i32,
		}
		#[repr(C)]
		#[derive(Clone, Copy)]
		pub struct DECIMAL {
			pub wReserved: u16,
			pub Anonymous: DECIMAL_0,
			pub Hi32: u32,
			pub Anonymous2: DECIMAL_1,
		}
		impl Default for DECIMAL {
			fn default() -> Self {
				unsafe { core::mem::zeroed() }
			}
		}
		#[repr(C)]
		#[derive(Clone, Copy)]
		pub union DECIMAL_0 {
			pub Anonymous: DECIMAL_0_0,
			pub signscale: u16,
		}
		impl Default for DECIMAL_0 {
			fn default() -> Self {
				unsafe { core::mem::zeroed() }
			}
		}
		#[repr(C)]
		#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
		pub struct DECIMAL_0_0 {
			pub scale: u8,
			pub sign: u8,
		}
		#[repr(C)]
		#[derive(Clone, Copy)]
		pub union DECIMAL_1 {
			pub Anonymous: DECIMAL_1_0,
			pub Lo64: u64,
		}
		impl Default for DECIMAL_1 {
			fn default() -> Self {
				unsafe { core::mem::zeroed() }
			}
		}
		#[repr(C)]
		#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
		pub struct DECIMAL_1_0 {
			pub Lo32: u32,
			pub Mid32: u32,
		}
		pub const ERROR_PIPE_NOT_CONNECTED: i32 = 233;
		pub const ERROR_SUCCESS: i32 = 0;
		pub const E_FAIL: windows_core::HRESULT = windows_core::HRESULT(0x80004005_u32 as _);
		pub const E_NOTIMPL: windows_core::HRESULT = windows_core::HRESULT(0x80004001_u32 as _);
		pub const E_UNEXPECTED: windows_core::HRESULT = windows_core::HRESULT(0x8000FFFF_u32 as _);
		#[repr(transparent)]
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
		pub struct HKEY(pub *mut core::ffi::c_void);
		pub const HKEY_CURRENT_USER: HKEY = HKEY(-2147483647 as _);
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
		windows_core::imp::define_interface!(
			IDispatch,
			IDispatch_Vtbl,
			0x00020400_0000_0000_c000_000000000046
		);
		windows_core::imp::interface_hierarchy!(IDispatch, windows_core::IUnknown);
		#[repr(C)]
		pub struct IDispatch_Vtbl {
			pub base__: windows_core::IUnknown_Vtbl,
			GetTypeInfoCount: usize,
			GetTypeInfo: usize,
			GetIDsOfNames: usize,
			Invoke: usize,
		}
		windows_core::imp::define_interface!(
			IErrorLog,
			IErrorLog_Vtbl,
			0x3127ca40_446e_11ce_8135_00aa004bb851
		);
		windows_core::imp::interface_hierarchy!(IErrorLog, windows_core::IUnknown);
		#[repr(C)]
		pub struct IErrorLog_Vtbl {
			pub base__: windows_core::IUnknown_Vtbl,
			AddError: usize,
		}
		windows_core::imp::define_interface!(
			IPropertyBag,
			IPropertyBag_Vtbl,
			0x55272a00_42cb_11ce_8135_00aa004bb851
		);
		windows_core::imp::interface_hierarchy!(IPropertyBag, windows_core::IUnknown);
		impl IPropertyBag {
			pub unsafe fn Read<P0, P2>(
				&self,
				pszpropname: P0,
				pvar: *mut VARIANT,
				perrorlog: P2,
			) -> windows_core::HRESULT
			where
				P0: windows_core::Param<windows_core::PCWSTR>,
				P2: windows_core::Param<IErrorLog>,
			{
				unsafe {
					(windows_core::Interface::vtable(self).Read)(
						windows_core::Interface::as_raw(self),
						pszpropname.param().abi(),
						pvar,
						perrorlog.param().abi(),
					)
				}
			}
			pub unsafe fn Write<P0>(
				&self,
				pszpropname: P0,
				pvar: *const VARIANT,
			) -> windows_core::HRESULT
			where
				P0: windows_core::Param<windows_core::PCWSTR>,
			{
				unsafe {
					(windows_core::Interface::vtable(self).Write)(
						windows_core::Interface::as_raw(self),
						pszpropname.param().abi(),
						pvar,
					)
				}
			}
		}
		#[repr(C)]
		pub struct IPropertyBag_Vtbl {
			pub base__: windows_core::IUnknown_Vtbl,
			pub Read: unsafe extern "system" fn(
				*mut core::ffi::c_void,
				windows_core::PCWSTR,
				*mut VARIANT,
				*mut core::ffi::c_void,
			) -> windows_core::HRESULT,
			pub Write: unsafe extern "system" fn(
				*mut core::ffi::c_void,
				windows_core::PCWSTR,
				*const VARIANT,
			) -> windows_core::HRESULT,
		}
		windows_core::imp::define_interface!(
			IRecordInfo,
			IRecordInfo_Vtbl,
			0x0000002f_0000_0000_c000_000000000046
		);
		windows_core::imp::interface_hierarchy!(IRecordInfo, windows_core::IUnknown);
		#[repr(C)]
		pub struct IRecordInfo_Vtbl {
			pub base__: windows_core::IUnknown_Vtbl,
			RecordInit: usize,
			RecordClear: usize,
			RecordCopy: usize,
			GetGuid: usize,
			GetName: usize,
			GetSize: usize,
			GetTypeInfo: usize,
			GetField: usize,
			GetFieldNoCopy: usize,
			PutField: usize,
			PutFieldNoCopy: usize,
			GetFieldNames: usize,
			IsMatchingType: usize,
			RecordCreate: usize,
			RecordCreateCopy: usize,
			RecordDestroy: usize,
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
		pub trait IWTSListener_Impl: windows_core::IUnknownImpl {
			fn GetConfiguration(&self) -> windows_core::Result<IPropertyBag>;
		}
		impl IWTSListener_Vtbl {
			pub const fn new<Identity: IWTSListener_Impl, const OFFSET: isize>() -> Self {
				unsafe extern "system" fn GetConfiguration<
					Identity: IWTSListener_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
					pppropertybag: *mut *mut core::ffi::c_void,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						match IWTSListener_Impl::GetConfiguration(this) {
							Ok(ok__) => {
								pppropertybag.write(core::mem::transmute(ok__));
								windows_core::HRESULT(0)
							}
							Err(err) => err.into(),
						}
					}
				}
				Self {
					base__: windows_core::IUnknown_Vtbl::new::<Identity, OFFSET>(),
					GetConfiguration: GetConfiguration::<Identity, OFFSET>,
				}
			}
			pub fn matches(iid: &windows_core::GUID) -> bool {
				iid == &<IWTSListener as windows_core::Interface>::IID
			}
		}
		impl windows_core::RuntimeName for IWTSListener {}
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
		pub trait IWTSVirtualChannel_Impl: windows_core::IUnknownImpl {
			fn Write(
				&self,
				cbsize: u32,
				pbuffer: *const u8,
				preserved: windows_core::Ref<windows_core::IUnknown>,
			) -> windows_core::Result<()>;
			fn Close(&self) -> windows_core::Result<()>;
		}
		impl IWTSVirtualChannel_Vtbl {
			pub const fn new<Identity: IWTSVirtualChannel_Impl, const OFFSET: isize>() -> Self {
				unsafe extern "system" fn Write<
					Identity: IWTSVirtualChannel_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
					cbsize: u32,
					pbuffer: *const u8,
					preserved: *mut core::ffi::c_void,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						IWTSVirtualChannel_Impl::Write(
							this,
							core::mem::transmute_copy(&cbsize),
							core::mem::transmute_copy(&pbuffer),
							core::mem::transmute_copy(&preserved),
						)
						.into()
					}
				}
				unsafe extern "system" fn Close<
					Identity: IWTSVirtualChannel_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						IWTSVirtualChannel_Impl::Close(this).into()
					}
				}
				Self {
					base__: windows_core::IUnknown_Vtbl::new::<Identity, OFFSET>(),
					Write: Write::<Identity, OFFSET>,
					Close: Close::<Identity, OFFSET>,
				}
			}
			pub fn matches(iid: &windows_core::GUID) -> bool {
				iid == &<IWTSVirtualChannel as windows_core::Interface>::IID
			}
		}
		impl windows_core::RuntimeName for IWTSVirtualChannel {}
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
		pub trait IWTSVirtualChannelManager_Impl: windows_core::IUnknownImpl {
			fn CreateListener(
				&self,
				pszchannelname: *const i8,
				uflags: u32,
				plistenercallback: windows_core::Ref<IWTSListenerCallback>,
			) -> windows_core::Result<IWTSListener>;
		}
		impl IWTSVirtualChannelManager_Vtbl {
			pub const fn new<Identity: IWTSVirtualChannelManager_Impl, const OFFSET: isize>() -> Self
			{
				unsafe extern "system" fn CreateListener<
					Identity: IWTSVirtualChannelManager_Impl,
					const OFFSET: isize,
				>(
					this: *mut core::ffi::c_void,
					pszchannelname: *const i8,
					uflags: u32,
					plistenercallback: *mut core::ffi::c_void,
					pplistener: *mut *mut core::ffi::c_void,
				) -> windows_core::HRESULT {
					unsafe {
						let this: &Identity =
							&*((this as *const *const ()).offset(OFFSET) as *const Identity);
						match IWTSVirtualChannelManager_Impl::CreateListener(
							this,
							core::mem::transmute_copy(&pszchannelname),
							core::mem::transmute_copy(&uflags),
							core::mem::transmute_copy(&plistenercallback),
						) {
							Ok(ok__) => {
								pplistener.write(core::mem::transmute(ok__));
								windows_core::HRESULT(0)
							}
							Err(err) => err.into(),
						}
					}
				}
				Self {
					base__: windows_core::IUnknown_Vtbl::new::<Identity, OFFSET>(),
					CreateListener: CreateListener::<Identity, OFFSET>,
				}
			}
			pub fn matches(iid: &windows_core::GUID) -> bool {
				iid == &<IWTSVirtualChannelManager as windows_core::Interface>::IID
			}
		}
		impl windows_core::RuntimeName for IWTSVirtualChannelManager {}
		pub const KEY_ALL_ACCESS: i32 = 983103;
		#[repr(transparent)]
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
		pub struct LSTATUS(pub i32);
		pub type REGSAM = ACCESS_MASK;
		#[repr(C)]
		#[derive(Clone, Copy, Debug, Eq, PartialEq)]
		pub struct SAFEARRAY {
			pub cDims: u16,
			pub fFeatures: u16,
			pub cbElements: u32,
			pub cLocks: u32,
			pub pvData: *mut core::ffi::c_void,
			pub rgsabound: [SAFEARRAYBOUND; 1],
		}
		impl Default for SAFEARRAY {
			fn default() -> Self {
				unsafe { core::mem::zeroed() }
			}
		}
		#[repr(C)]
		#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
		pub struct SAFEARRAYBOUND {
			pub cElements: u32,
			pub lLbound: i32,
		}
		#[repr(transparent)]
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
		pub struct SCODE(pub i32);
		#[repr(C)]
		pub struct VARIANT {
			pub Anonymous: VARIANT_0,
		}
		impl Clone for VARIANT {
			fn clone(&self) -> Self {
				unsafe { core::mem::transmute_copy(self) }
			}
		}
		impl Default for VARIANT {
			fn default() -> Self {
				unsafe { core::mem::zeroed() }
			}
		}
		#[repr(C)]
		pub union VARIANT_0 {
			pub Anonymous: core::mem::ManuallyDrop<VARIANT_0_0>,
			pub decVal: DECIMAL,
		}
		impl Clone for VARIANT_0 {
			fn clone(&self) -> Self {
				unsafe { core::mem::transmute_copy(self) }
			}
		}
		impl Default for VARIANT_0 {
			fn default() -> Self {
				unsafe { core::mem::zeroed() }
			}
		}
		#[repr(C)]
		pub struct VARIANT_0_0 {
			pub vt: VARTYPE,
			pub wReserved1: u16,
			pub wReserved2: u16,
			pub wReserved3: u16,
			pub Anonymous: VARIANT_0_0_0,
		}
		impl Clone for VARIANT_0_0 {
			fn clone(&self) -> Self {
				unsafe { core::mem::transmute_copy(self) }
			}
		}
		impl Default for VARIANT_0_0 {
			fn default() -> Self {
				unsafe { core::mem::zeroed() }
			}
		}
		#[repr(C)]
		pub union VARIANT_0_0_0 {
			pub llVal: i64,
			pub lVal: i32,
			pub bVal: u8,
			pub iVal: i16,
			pub fltVal: f32,
			pub dblVal: f64,
			pub boolVal: VARIANT_BOOL,
			pub __OBSOLETE__VARIANT_BOOL: VARIANT_BOOL,
			pub scode: SCODE,
			pub cyVal: CY,
			pub date: f64,
			pub bstrVal: core::mem::ManuallyDrop<windows_core::BSTR>,
			pub punkVal: core::mem::ManuallyDrop<Option<windows_core::IUnknown>>,
			pub pdispVal: core::mem::ManuallyDrop<Option<IDispatch>>,
			pub parray: *mut SAFEARRAY,
			pub pbVal: *mut u8,
			pub piVal: *mut i16,
			pub plVal: *mut i32,
			pub pllVal: *mut i64,
			pub pfltVal: *mut f32,
			pub pdblVal: *mut f64,
			pub pboolVal: *mut VARIANT_BOOL,
			pub __OBSOLETE__VARIANT_PBOOL: *mut VARIANT_BOOL,
			pub pscode: *mut SCODE,
			pub pcyVal: *mut CY,
			pub pdate: *mut f64,
			pub pbstrVal: *mut windows_core::BSTR,
			pub ppunkVal: *mut Option<windows_core::IUnknown>,
			pub ppdispVal: *mut Option<IDispatch>,
			pub pparray: *mut *mut SAFEARRAY,
			pub pvarVal: *mut VARIANT,
			pub byref: *mut core::ffi::c_void,
			pub cVal: i8,
			pub uiVal: u16,
			pub ulVal: u32,
			pub ullVal: u64,
			pub intVal: i32,
			pub uintVal: u32,
			pub pdecVal: *mut DECIMAL,
			pub pcVal: *mut i8,
			pub puiVal: *mut u16,
			pub pulVal: *mut u32,
			pub pullVal: *mut u64,
			pub pintVal: *mut i32,
			pub puintVal: *mut u32,
			pub Anonymous: core::mem::ManuallyDrop<VARIANT_0_0_0_0>,
		}
		impl Clone for VARIANT_0_0_0 {
			fn clone(&self) -> Self {
				unsafe { core::mem::transmute_copy(self) }
			}
		}
		impl Default for VARIANT_0_0_0 {
			fn default() -> Self {
				unsafe { core::mem::zeroed() }
			}
		}
		#[repr(C)]
		#[derive(Clone, Debug, Default, Eq, PartialEq)]
		pub struct VARIANT_0_0_0_0 {
			pub pvRecord: *mut core::ffi::c_void,
			pub pRecInfo: core::mem::ManuallyDrop<Option<IRecordInfo>>,
		}
		#[repr(transparent)]
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
		pub struct VARIANT_BOOL(pub i16);
		#[repr(transparent)]
		#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
		pub struct VARTYPE(pub u16);
	}
}
