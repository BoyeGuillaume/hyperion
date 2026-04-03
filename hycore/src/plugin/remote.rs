use crate::{
    api::ext::remote::StartRemoteServerInfo,
    hyerror, hyinfo, hywarn,
    instance::plugin::Plugin,
    plugin::logger::{LoggerExt, LoggerStateRes},
    register_plugin,
    tokio::TokioRuntime,
};
use bevy_ecs::{prelude::*, system::SystemId};
use tokio_util::sync::CancellationToken;

pub mod server;

/// A plugin that allows remote communication with the instance.
///
/// Can be used to create a remote control interface, or to expose the instance's functionality to other applications.
#[derive(Debug, Clone, Default)]
pub struct RemotePlugin;

impl Plugin for RemotePlugin {
    fn is_public(&self) -> bool {
        true
    }

    fn init(
        &mut self,
        instance: &mut crate::instance::Instance,
        _ext: Option<&mut crate::ext::ExtList>,
    ) -> crate::HyResult<()> {
        // Register both start_webserver and stop_webserver systems to run as part of the startup schedule.
        let start_webserver_cache = instance.world.register_system_cached(start_webserver);
        let stop_webserver_cache = instance.world.register_system_cached(stop_webserver);

        instance.world.insert_resource(RemoteServerSystemCache {
            start_webserver: start_webserver_cache,
            stop_webserver: stop_webserver_cache,
        });

        Ok(())
    }
}
register_plugin!(RemotePlugin);

#[derive(Resource)]
pub struct RemoteServerSystemCache {
    pub start_webserver: SystemId<In<StartRemoteServerInfo>>,
    pub stop_webserver: SystemId,
}

// #[derive(Resource, Clone)]
// struct ResRemoteCreateInfo {
//     create_info: RemotePluginCreateInfo,
//     cert_info: Option<TlsServerCertificateInfo>,
//     client_auth_info: Option<TlsClientAuthInfo>,
// }

#[derive(Resource)]
struct RemoteCancellationToken(CancellationToken);

/// A system that starts the remote server. This should only be run once.
fn start_webserver(
    In(create_info): In<StartRemoteServerInfo>,
    rt: Res<TokioRuntime>,
    cancellation_token: Option<Res<RemoteCancellationToken>>,
    logger: Res<LoggerStateRes>,
    mut commands: Commands,
) {
    if cancellation_token.is_some() {
        hywarn!(logger;
            "Remote server is already running. Skipping startup. To restart the server, stop it and start it again."
        );
        return;
    }

    // Start the server in a new Tokio task
    let cancellation_token = CancellationToken::new();
    let cancellation_token_2 = cancellation_token.clone();

    {
        let logger = logger.logger();
        let _guard = rt.enter();
        tokio::spawn(async move {
            if let Err(e) = server::run_server(&logger, create_info, cancellation_token_2).await {
                hyerror!(logger; "Remote server error: {:?}", e);
            } else {
                hyinfo!(logger; "Remote server stopped gracefully.");
            }
        });
    }

    commands.insert_resource(RemoteCancellationToken(cancellation_token));
}

/// Stop the remote server if it is running.
fn stop_webserver(
    cancellation_token: Option<Res<RemoteCancellationToken>>,
    logger: Res<LoggerStateRes>,
    mut commands: Commands,
) {
    if cancellation_token.is_none() {
        hywarn!(logger; "Remote server is not running. Skipping shutdown.");
        return;
    }

    hyinfo!(logger; "Stopping remote server...");
    let cancellation_token = cancellation_token.unwrap();
    cancellation_token.0.cancel();
    commands.remove_resource::<RemoteCancellationToken>();
}
