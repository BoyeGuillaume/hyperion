use actix_web::{
    App, HttpResponse, HttpServer, Responder, dev::Extensions, get, http::header::ContentType,
    middleware,
};
use std::net::SocketAddr;
use tokio_util::{future::FutureExt, sync::CancellationToken};

use crate::{
    HyResult,
    api::ext::remote::{TlsClientAuthentificationInfo, TlsServerCertificateInfo},
    hyerror, hyinfo,
    plugin::{logger::Logger, remote::StartRemoteServerInfo},
};

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub bind: SocketAddr,
    pub peer: SocketAddr,
    pub ttl: Option<u32>,
}

#[get("/")]
async fn index() -> impl Responder {
    let html_content = include_str!("index.html");

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_content)
}

#[get("/favicon.ico")]
async fn favicon() -> impl Responder {
    let ico_content = include_bytes!("../../../../assets/favicon.ico");

    HttpResponse::Ok()
        .content_type("image/x-icon")
        .body(ico_content.as_ref())
}

pub fn run_server(
    logger: &Logger,
    mut create_info: StartRemoteServerInfo,
    cancellation_token: CancellationToken,
) -> impl Future<Output = HyResult<()>> + Send + use<'_> {
    async move {
        let addr = (create_info.host, create_info.port);
        hyinfo!(logger; "Starting remote plugin server on {}:{}", &create_info.host, create_info.port);

        let server = HttpServer::new(move || {
            let app = App::new()
                .wrap(middleware::Compress::default())
                .service(index)
                .service(favicon);
            app
        })
        .workers(1)
        .max_connections(create_info.max_connections)
        .disable_signals();

        // Extract TLS configuration from the ext list, if provided
        let tls_server_certificate_info = create_info.ext.pop::<TlsServerCertificateInfo>();
        let tls_client_authentification_config =
            create_info.ext.pop::<TlsClientAuthentificationInfo>();
        if !create_info.ext.is_empty() {
            hyerror!(logger; "Unused extensions in LaunchRemoteServerInfo: {:?}", create_info.ext);
        }

        if tls_client_authentification_config.is_some() {
            anyhow::bail!(
                "TLS client authentication configuration provided for remote server, but TLS client authentication is not currently supported. Please remove the TLS client authentication configuration to start the server."
            );
        }

        let server = if tls_server_certificate_info.is_some()
            || tls_client_authentification_config.is_some()
        {
            #[cfg(feature = "remote-tls")]
            {
                use anyhow::Context;
                use rustls::{
                    RootCertStore,
                    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
                    server::WebPkiClientVerifier,
                };
                use std::sync::Arc;

                use crate::{hydebug, hytrace};

                hytrace!(logger; "Initializing crypto provider based on aws-lc-rs");
                let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
                let tls_server_config = match tls_server_certificate_info {
                    Some(config) => config,
                    None => anyhow::bail!(
                        "TLS client authentication configuration provided without server certificate configuration. Please provide a server certificate configuration to use TLS functionality."
                    ),
                };

                // Load the TLS key/cert files
                hydebug!(logger; "Loading TLS server certificate from {}", tls_server_config.cert_pem_path);
                let cert_chain =
                    CertificateDer::pem_file_iter(tls_server_config.cert_pem_path.as_str())
                        .with_context(|| {
                            format!(
                                "Failed to read TLS server certificate from path: {}",
                                tls_server_config.cert_pem_path
                            )
                        })?
                        .flatten()
                        .collect::<Vec<_>>();

                hydebug!(logger; "Loading TLS server private key from {}", tls_server_config.key_pem_path);
                let key_der = PrivateKeyDer::from_pem_file(tls_server_config.key_pem_path.as_str())
                    .with_context(|| {
                        format!(
                            "Failed to read TLS server private key from path: {}",
                            tls_server_config.key_pem_path
                        )
                    })?;

                let mut client_auth = None;
                if let Some(client_auth_config) = tls_client_authentification_config {
                    use crate::hydebug;

                    hydebug!(logger; "Configuring TLS client authentication with root CA from {}", client_auth_config.root_ca_pem_path);
                    let mut root_store = RootCertStore::empty();

                    CertificateDer::pem_file_iter(client_auth_config.root_ca_pem_path.as_str())
                        .with_context(|| {
                            format!(
                                "Failed to read TLS client authentication root CA certificate from path: {}",
                                client_auth_config.root_ca_pem_path
                            )
                        })?
                        .flatten()
                        .try_for_each(|cert| {
                            root_store
                                .add(cert)
                                .map_err(|e| anyhow::anyhow!("Failed to add client auth root CA certificate: {:?}", e))
                        })?;

                    hydebug!(logger; "Successfully loaded {} certificates into TLS client authentication root store", root_store.len());
                    client_auth = Some(
                        WebPkiClientVerifier::builder_with_provider(
                            Arc::new(root_store),
                            provider.clone(),
                        )
                        .build()
                        .with_context(|| "Failed to create TLS client authentication verifier")?,
                    );
                }

                let config_builder = if let Some(client_auth) = client_auth.as_ref() {
                    rustls::ServerConfig::builder_with_provider(provider.clone())
                        .with_safe_default_protocol_versions()
                        .with_context(|| "Failed to create TLS ServerConfig")?
                        .with_client_cert_verifier(client_auth.clone())
                } else {
                    rustls::ServerConfig::builder_with_provider(provider.clone())
                        .with_safe_default_protocol_versions()
                        .with_context(|| "Failed to create TLS ServerConfig")?
                        .with_no_client_auth()
                };

                let config = config_builder
                    .with_single_cert(cert_chain, key_der)
                    .with_context(|| {
                        format!(
                            "Failed to create TLS server configuration with cert: {} and key: {}",
                            tls_server_config.cert_pem_path, tls_server_config.key_pem_path
                        )
                    })?;

                hydebug!(logger; "Starting TLS-enabled server on {}:{}", create_info.host, create_info.port);
                server.bind_rustls_0_23(addr, config)?
            }
            #[cfg(not(feature = "remote-tls"))]
            {
                hyerror!(logger; "TLS configuration provided for remote server, but the server was not compiled with TLS support. Please enable the \"remote-tls\" feature to use TLS functionality.");
                anyhow::bail!(
                    "TLS configuration provided for remote server, but the server was not compiled with TLS support. Please enable the \"remote-tls\" feature to use TLS functionality."
                );
            }
        } else {
            use crate::hydebug;
            hydebug!(logger; "Starting non-TLS server on {}:{}", create_info.host, create_info.port);
            server.bind(addr)?
        };

        server
            .on_connect(get_client_cert)
            .run()
            .with_cancellation_token_owned(cancellation_token)
            .await
            .map(|x| x.map_err(|err| anyhow::anyhow!(err)))
            .unwrap_or_else(|| {
                Err(anyhow::anyhow!(
                    "Failed to start remote server on {}:{}. Please check if the address is correct and not already in use.",
                    create_info.host, create_info.port
                ))
            })?;

        Ok(())
    }
}

fn get_client_cert(connection: &'_ dyn std::any::Any, data: &'_ mut Extensions) {
    #[cfg(feature = "remote-tls")]
    if let Some(tls_socket) = connection
        .downcast_ref::<actix_tls::accept::rustls_0_23::TlsStream<actix_web::rt::net::TcpStream>>()
    {
        let (socket, tls_session) = tls_socket.get_ref();
        data.insert(ConnectionInfo {
            bind: socket.local_addr().unwrap(),
            peer: socket.peer_addr().unwrap(),
            ttl: socket.ttl().ok(),
        });

        if let Some(certs) = tls_session.peer_certificates() {
            if let Some(client_cert) = certs.first() {
                data.insert(client_cert.clone());
            }
        }

        return;
    }

    if let Some(socket) = connection.downcast_ref::<actix_web::rt::net::TcpStream>() {
        data.insert(ConnectionInfo {
            bind: socket.local_addr().unwrap(),
            peer: socket.peer_addr().unwrap(),
            ttl: socket.ttl().ok(),
        });
    } else {
        unreachable!(
            "Connection is neither a TLS stream nor a plain TCP stream, cannot extract client certificate information"
        );
    }
}
