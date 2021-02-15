use crate::error::Error;
use alloc::rc::Rc;
use core::convert::TryInto;
use s2n_tls_sys::*;
use std::ffi::CString;

struct Owned(*mut s2n_config);

impl Default for Owned {
    fn default() -> Self {
        Self::new()
    }
}

impl Owned {
    fn new() -> Self {
        crate::init::init();
        let config = call!(s2n_config_new()).unwrap();
        Self(config)
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut s2n_config {
        self.0
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        let _ = call!(s2n_config_free(self.0));
    }
}

#[derive(Clone, Default)]
pub struct Config(Rc<Owned>);

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> Builder {
        Builder::default()
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut s2n_config {
        (self.0).0
    }
}

#[derive(Default)]
pub struct Builder(Owned);

impl Builder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_alert_behavior(&mut self, value: s2n_alert_behavior) -> Result<&mut Self, Error> {
        call!(s2n_config_set_alert_behavior(self.as_mut_ptr(), value))?;
        Ok(self)
    }

    pub fn set_cipher_preference(&mut self, name: &str) -> Result<&mut Self, Error> {
        let name = CString::new(name).map_err(|_| Error::InvalidInput)?;
        call!(s2n_config_set_cipher_preferences(
            self.as_mut_ptr(),
            name.as_ptr() as *const _
        ))?;
        Ok(self)
    }

    /// sets the application protocol preferences on an s2n_config object.
    ///
    /// protocols is a list in order of preference, with most preferred protocol first,
    /// and of length protocol_count. When acting as an S2N_CLIENT the protocol list is
    /// included in the Client Hello message as the ALPN extension. As an S2N_SERVER, the
    /// list is used to negotiate a mutual application protocol with the client. After
    /// the negotiation for the connection has completed, the agreed upon protocol can
    /// be retrieved with s2n_get_application_protocol
    pub fn set_alpn_preference<P: IntoIterator<Item = I>, I: AsRef<[u8]>>(
        &mut self,
        protocols: P,
    ) -> Result<(), Error> {
        // reset the list
        call!(s2n_config_set_protocol_preferences(
            self.as_mut_ptr(),
            core::ptr::null(),
            0
        ))?;

        for protocol in protocols {
            self.append_alpn_preference(protocol.as_ref())?;
        }

        Ok(())
    }

    pub fn load_pem(&mut self, certificate: &[u8], private_key: &[u8]) -> Result<&mut Self, Error> {
        let certificate = CString::new(certificate).map_err(|_| Error::InvalidInput)?;
        let private_key = CString::new(private_key).map_err(|_| Error::InvalidInput)?;
        call!(s2n_config_add_cert_chain_and_key(
            self.as_mut_ptr(),
            certificate.as_ptr(),
            private_key.as_ptr()
        ))?;
        Ok(self)
    }

    pub fn trust_pem(&mut self, certificate: &[u8]) -> Result<&mut Self, Error> {
        let certificate = CString::new(certificate).map_err(|_| Error::InvalidInput)?;
        call!(s2n_config_add_pem_to_trust_store(
            self.as_mut_ptr(),
            certificate.as_ptr(),
        ))?;
        Ok(self)
    }

    pub fn append_alpn_preference(&mut self, protocol: &[u8]) -> Result<&mut Self, Error> {
        call!(s2n_config_append_protocol_preference(
            self.as_mut_ptr(),
            protocol.as_ptr(),
            protocol.len().try_into().map_err(|_| Error::InvalidInput)?,
        ))?;
        Ok(self)
    }

    /// # Safety
    ///
    /// The `context` pointer must live at least as long as the config
    pub unsafe fn set_verify_host_callback(
        &mut self,
        callback: s2n_verify_host_fn,
        context: *mut core::ffi::c_void,
    ) -> Result<&mut Self, Error> {
        call!(s2n_config_set_verify_host_callback(
            self.as_mut_ptr(),
            callback,
            context
        ))?;
        Ok(self)
    }

    pub fn build(self) -> Result<Config, Error> {
        Ok(Config(Rc::new(self.0)))
    }

    fn as_mut_ptr(&mut self) -> *mut s2n_config {
        self.0.as_mut_ptr()
    }
}

#[cfg(feature = "quic")]
impl Builder {
    pub fn enable_quic(&mut self) -> Result<&mut Self, Error> {
        call!(s2n_tls_sys::s2n_config_enable_quic(self.as_mut_ptr()))?;
        Ok(self)
    }
}