use std::net::IpAddr;

use crate::{
    HyResult,
    ext::{ExtList, ExtObject},
    hyerror,
    instance::Instance,
};

/// Extension for TLS server certificate information
#[derive(Debug, Clone)]
pub struct TlsServerCertificateInfo {
    /// Path to the TLS certificate file (PEM format).
    pub cert_pem_path: String,

    /// Path to the TLS private key file (PEM format).
    pub key_pem_path: String,
}
impl ExtObject for TlsServerCertificateInfo {}

/// Extension for TLS client authentication information
#[derive(Debug, Clone)]
pub struct TlsClientAuthentificationInfo {
    /// Root CA certificate path for verifying client certificates (PEM format).
    pub root_ca_pem_path: String,
}
impl ExtObject for TlsClientAuthentificationInfo {}

/// Extension for remote plugin configuration.
#[derive(Debug)]
pub struct StartRemoteServerInfo {
    /// The port to listen on for incoming connections.
    pub port: u16,

    /// Host to bind to (IPv4 or IPv6).
    pub host: IpAddr,

    /// Maximum number of concurrent connections allowed (default is 2048).
    pub max_connections: usize,

    /// Addition ext-list for extensions related to remote plugin configuration.
    pub ext: ExtList,
}

#[cfg(feature = "cffi")]
mod cffi {
    use crate::{
        api::cffi::{HyInstance, function::cffi_return_error, r#struct::HyStructureType},
        ext::ExtObjectCFFIInventory,
    };

    use super::*;

    #[derive(Clone, Copy)]
    #[repr(C)]
    pub struct HyStartRemoteServerInfo {
        pub s_type: HyStructureType,
        pub port: u16,
        pub p_host: *const std::ffi::c_char,
        pub max_connections: usize,
        pub p_next: *mut std::ffi::c_void,
    }

    impl HyStartRemoteServerInfo {
        pub unsafe fn to_rust(&self) -> HyResult<StartRemoteServerInfo> {
            // Parse the host string from C string to Rust String, then to IpAddr
            // if null, default to LOCALHOST (IpAddr::V4(Ipv4Addr::LOCALHOST))
            let host = if self.p_host.is_null() {
                IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
            } else {
                let host_str = unsafe { std::ffi::CStr::from_ptr(self.p_host) }.to_str()?;
                host_str.parse::<IpAddr>()?
            };

            Ok(StartRemoteServerInfo {
                port: self.port,
                host,
                max_connections: self.max_connections,
                ext: unsafe { ExtList::from_cffi(self.p_next) }?,
            })
        }
    }

    #[derive(Clone, Copy)]
    #[repr(C)]
    pub struct HyTlsServerCertificateInfo {
        pub s_type: HyStructureType,
        pub p_cert_pem_path: *const std::ffi::c_char,
        pub p_key_pem_path: *const std::ffi::c_char,
        pub p_next: *mut std::ffi::c_void,
    }

    impl HyTlsServerCertificateInfo {
        pub unsafe fn to_rust(&self) -> HyResult<TlsServerCertificateInfo> {
            if self.p_cert_pem_path.is_null() || self.p_key_pem_path.is_null() {
                anyhow::bail!("Certificate PEM path and key PEM path cannot be null.");
            }

            let cert_pem_path = unsafe { std::ffi::CStr::from_ptr(self.p_cert_pem_path) }
                .to_str()?
                .to_string();

            let key_pem_path = unsafe { std::ffi::CStr::from_ptr(self.p_key_pem_path) }
                .to_str()?
                .to_string();

            Ok(TlsServerCertificateInfo {
                cert_pem_path,
                key_pem_path,
            })
        }
    }

    #[derive(Clone, Copy)]
    #[repr(C)]
    pub struct HyTlsClientAuthentificationInfo {
        pub s_type: HyStructureType,
        pub p_root_ca_pem_path: *const std::ffi::c_char,
        pub p_next: *mut std::ffi::c_void,
    }

    impl HyTlsClientAuthentificationInfo {
        pub unsafe fn to_rust(&self) -> HyResult<TlsClientAuthentificationInfo> {
            if self.p_root_ca_pem_path.is_null() {
                anyhow::bail!("Root CA PEM path cannot be null.");
            }

            let root_ca_pem_path =
                unsafe { std::ffi::CStr::from_ptr(self.p_root_ca_pem_path as *const i8) }
                    .to_str()?
                    .to_string();

            Ok(TlsClientAuthentificationInfo { root_ca_pem_path })
        }
    }

    inventory::submit! {
        ExtObjectCFFIInventory {
            stype: HyStructureType::TlsClientAuthentificationInfo as u32,
            callback: |ptr| {
                let create_info = unsafe { *(ptr as *const HyTlsClientAuthentificationInfo) };
                let p_next = create_info.p_next;
                let tls_client_auth_info = unsafe { create_info.to_rust() }?;
                Ok((Box::new(tls_client_auth_info) as Box<dyn ExtObject>, p_next))
            },
        }
    }

    inventory::submit! {
        ExtObjectCFFIInventory {
            stype: HyStructureType::TlsServerCertificateInfo as u32,
            callback: |ptr| {
                let create_info = unsafe { *(ptr as *const HyTlsServerCertificateInfo) };
                let p_next = create_info.p_next;
                let tls_server_cert_info = unsafe { create_info.to_rust() }?;
                Ok((Box::new(tls_server_cert_info) as Box<dyn ExtObject>, p_next))
            },
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn hyStartRemoteServer(
        p_instance: *mut HyInstance,
        p_create_info: *const HyStartRemoteServerInfo,
    ) -> std::ffi::c_int {
        if p_instance.is_null() {
            return cffi_return_error(anyhow::format_err!("Instance pointer cannot be null."));
        }

        if p_create_info.is_null() {
            return cffi_return_error(anyhow::format_err!("Create info pointer cannot be null."));
        }

        let instance = unsafe { &mut *(p_instance as *mut Instance) };
        let create_info = unsafe { &*p_create_info };
        let create_info = match unsafe { create_info.to_rust() } {
            Ok(info) => info,
            Err(e) => return cffi_return_error(e),
        };

        match super::hy_start_remote_server(instance, create_info) {
            Ok(_) => 0,
            Err(e) => cffi_return_error(e),
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn hyShutdownRemoteServer(p_instance: *mut HyInstance) -> std::ffi::c_int {
        if p_instance.is_null() {
            return cffi_return_error(anyhow::format_err!("Instance pointer cannot be null."));
        }

        let instance = unsafe { &mut *(p_instance as *mut Instance) };
        match super::hy_shutdown_remote_server(instance) {
            Ok(_) => 0,
            Err(e) => cffi_return_error(e),
        }
    }
}

pub fn hy_start_remote_server(
    instance: &mut Instance,
    create_info: StartRemoteServerInfo,
) -> HyResult<()> {
    #[cfg(feature = "remote")]
    {
        let system_cache =
            instance.get_resource::<crate::plugin::remote::RemoteServerSystemCache>();
        if system_cache.is_none() {
            hyerror!(instance; "RemotePlugin is not initialized. Please add RemotePlugin to the instance's plugin list before launching the remote server.");
            anyhow::bail!(
                "RemotePlugin is not initialized. Please add RemotePlugin to the instance's plugin list before launching the remote server."
            );
        }
        let system_cache = system_cache.unwrap();

        instance
            .world
            .run_system_with(system_cache.start_webserver, create_info)
            .inspect_err(|e| {
                hyerror!(instance; "Failed to launch remote server: {:#}", e);
            })?;
        Ok(())
    }
    #[cfg(not(feature = "remote"))]
    {
        hyerror!(instance; "Remote plugin feature is not enabled. Please enable the 'remote' feature to use remote server functionality.");
        anyhow::bail!(
            "Remote plugin feature is not enabled. Please enable the 'remote' feature to use remote server functionality."
        );
    }
}

pub fn hy_shutdown_remote_server(instance: &mut Instance) -> HyResult<()> {
    #[cfg(feature = "remote")]
    {
        let system_cache =
            instance.get_resource::<crate::plugin::remote::RemoteServerSystemCache>();
        if system_cache.is_none() {
            hyerror!(instance; "RemotePlugin is not initialized. Please add RemotePlugin to the instance's plugin list before stopping the remote server.");
            anyhow::bail!(
                "RemotePlugin is not initialized. Please add RemotePlugin to the instance's plugin list before stopping the remote server."
            );
        }
        let system_cache = system_cache.unwrap();

        instance
            .world
            .run_system(system_cache.stop_webserver)
            .inspect_err(|e| {
                hyerror!(instance; "Failed to stop remote server: {:#}", e);
            })?;
        Ok(())
    }
    #[cfg(not(feature = "remote"))]
    {
        hyerror!(instance; "Remote plugin feature is not enabled. Please enable the 'remote' feature to use remote server functionality.");
        anyhow::bail!(
            "Remote plugin feature is not enabled. Please enable the 'remote' feature to use remote server functionality."
        );
    }
}
