pub mod sys {
    extern crate libc;

    #[repr(C)]
    pub struct virDomain {}

    pub type virDomainPtr = *mut virDomain;
}
