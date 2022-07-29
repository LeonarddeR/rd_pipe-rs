use windows::{
    core::implement,
    Win32::System::Com::{IClassFactory, IClassFactory_Impl}
};

#[implement(IClassFactory)]
struct ClassFactory();

impl IClassFactory_Impl for ClassFactory {}
