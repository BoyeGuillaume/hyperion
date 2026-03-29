use crate::{
    ext::ExtObject,
    hydebug, hyerror, hyinfo,
    instance::plugin::Plugin,
    plugin::logger::LoggerStateRes,
    register_plugin,
    schedule::{Last, PreStartup, ScheduleOrder, Startup},
};
use async_channel::{Receiver, Sender};
use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
use bevy_remote::{
    BrpError, BrpResult, RemoteMethodHandler, RemoteMethodSystemId, RemoteMethods,
    RemoteWatchingMethodSystemId, builtin_methods, error_codes,
    http::{HostAddress, HostPort},
    schemas,
};
use bevy_tasks::IoTaskPool;
use hyper::header::{HeaderName, HeaderValue};
use std::{collections::HashMap, net::IpAddr};

#[derive(Debug)]
pub struct RemotePluginCreateInfo {
    pub port: u16,
    pub host: IpAddr,
}

impl ExtObject for RemotePluginCreateInfo {}

#[cfg(feature = "cffi")]
pub mod cffi {
    use std::net::IpAddr;

    use crate::{
        api::cffi::r#struct::HyStructureType,
        ext::{ExtObject, ExtObjectCFFIInventory},
        plugin::remote::RemotePluginCreateInfo,
    };

    #[derive(Clone, Copy)]
    #[repr(C)]
    pub struct HyRemotePluginCreateInfo {
        pub s_type: HyStructureType,
        pub port: u16,
        pub host: *const std::ffi::c_char,
        pub p_next: *mut std::ffi::c_void,
    }

    impl TryFrom<HyRemotePluginCreateInfo> for RemotePluginCreateInfo {
        type Error = anyhow::Error;

        fn try_from(value: HyRemotePluginCreateInfo) -> Result<Self, Self::Error> {
            if value.s_type != HyStructureType::RemotePluginCreateInfo {
                anyhow::bail!(
                    "Invalid structure type: expected `RemotePluginCreateInfo` ({:?}), got {:?}",
                    HyStructureType::RemotePluginCreateInfo,
                    value.s_type
                );
            }

            let host_cstr = unsafe {
                if value.host.is_null() {
                    anyhow::bail!("Host pointer cannot be null in `RemotePluginCreateInfo`");
                }
                std::ffi::CStr::from_ptr(value.host)
            };

            let host_str = host_cstr.to_str().map_err(|err| {
                anyhow::anyhow!(
                    "Failed to convert host C string to Rust string in `RemotePluginCreateInfo`: {err}"
                )
            })?;

            let host_ip = host_str.parse::<IpAddr>().map_err(|err| {
                anyhow::anyhow!(
                    "Failed to parse host string as IP address in `RemotePluginCreateInfo`: {err}"
                )
            })?;

            Ok(Self {
                port: value.port,
                host: host_ip,
            })
        }
    }

    inventory::submit! {
        ExtObjectCFFIInventory {
            stype: HyStructureType::RemotePluginCreateInfo as u32,
            callback: |ptr| {
                let create_info = unsafe { *(ptr as *const HyRemotePluginCreateInfo) };
                let p_next = create_info.p_next;
                let remote_create_info = RemotePluginCreateInfo::try_from(create_info)?;
                Ok((Box::new(remote_create_info) as Box<dyn ExtObject>, p_next))
            },
        }
    }
}

/// Remote plugin for Bevy that allows clients to send JSON-RPC requests to the Bevy application
pub struct RemotePlugin {
    methods: Vec<(String, RemoteMethodHandler)>,

    address: IpAddr,
    port: u16,
    headers: Headers,
}

impl RemotePlugin {
    pub fn empty() -> Self {
        Self {
            methods: Vec::new(),
            address: bevy_remote::http::DEFAULT_ADDR,
            port: bevy_remote::http::DEFAULT_PORT,
            headers: Headers::default(),
        }
    }

    /// Add a remote method to the plugin using the given `name` and `handler`.
    #[must_use]
    pub fn with_method<M>(
        mut self,
        name: impl Into<String>,
        handler: impl IntoSystem<In<Option<serde_json::Value>>, BrpResult, M>,
    ) -> Self {
        self.methods.push((
            name.into(),
            RemoteMethodHandler::Instant(Box::new(IntoSystem::into_system(handler))),
        ));
        self
    }

    /// Add a remote method with a watching handler to the plugin using the given `name`.
    #[must_use]
    pub fn with_watching_method<M>(
        mut self,
        name: impl Into<String>,
        handler: impl IntoSystem<In<Option<serde_json::Value>>, BrpResult<Option<serde_json::Value>>, M>,
    ) -> Self {
        self.methods.push((
            name.into(),
            RemoteMethodHandler::Watching(Box::new(IntoSystem::into_system(handler))),
        ));
        self
    }
}

impl std::default::Default for RemotePlugin {
    fn default() -> Self {
        Self::empty()
            .with_method(
                builtin_methods::BRP_GET_COMPONENTS_METHOD,
                builtin_methods::process_remote_get_components_request,
            )
            .with_method(
                builtin_methods::BRP_QUERY_METHOD,
                builtin_methods::process_remote_query_request,
            )
            .with_method(
                builtin_methods::BRP_SPAWN_ENTITY_METHOD,
                builtin_methods::process_remote_spawn_entity_request,
            )
            .with_method(
                builtin_methods::BRP_INSERT_COMPONENTS_METHOD,
                builtin_methods::process_remote_insert_components_request,
            )
            .with_method(
                builtin_methods::BRP_REMOVE_COMPONENTS_METHOD,
                builtin_methods::process_remote_remove_components_request,
            )
            .with_method(
                builtin_methods::BRP_DESPAWN_COMPONENTS_METHOD,
                builtin_methods::process_remote_despawn_entity_request,
            )
            .with_method(
                builtin_methods::BRP_REPARENT_ENTITIES_METHOD,
                builtin_methods::process_remote_reparent_entities_request,
            )
            .with_method(
                builtin_methods::BRP_LIST_COMPONENTS_METHOD,
                builtin_methods::process_remote_list_components_request,
            )
            .with_method(
                builtin_methods::BRP_MUTATE_COMPONENTS_METHOD,
                builtin_methods::process_remote_mutate_components_request,
            )
            .with_method(
                builtin_methods::RPC_DISCOVER_METHOD,
                builtin_methods::process_remote_list_methods_request,
            )
            .with_watching_method(
                builtin_methods::BRP_GET_COMPONENTS_AND_WATCH_METHOD,
                builtin_methods::process_remote_get_components_watching_request,
            )
            .with_watching_method(
                builtin_methods::BRP_LIST_COMPONENTS_AND_WATCH_METHOD,
                builtin_methods::process_remote_list_components_watching_request,
            )
            .with_method(
                builtin_methods::BRP_GET_RESOURCE_METHOD,
                builtin_methods::process_remote_get_resources_request,
            )
            .with_method(
                builtin_methods::BRP_INSERT_RESOURCE_METHOD,
                builtin_methods::process_remote_insert_resources_request,
            )
            .with_method(
                builtin_methods::BRP_REMOVE_RESOURCE_METHOD,
                builtin_methods::process_remote_remove_resources_request,
            )
            .with_method(
                builtin_methods::BRP_MUTATE_RESOURCE_METHOD,
                builtin_methods::process_remote_mutate_resources_request,
            )
            .with_method(
                builtin_methods::BRP_LIST_RESOURCES_METHOD,
                builtin_methods::process_remote_list_resources_request,
            )
            .with_method(
                builtin_methods::BRP_TRIGGER_EVENT_METHOD,
                builtin_methods::process_remote_trigger_event_request,
            )
            .with_method(
                builtin_methods::BRP_REGISTRY_SCHEMA_METHOD,
                builtin_methods::export_registry_types,
            )
    }
}

impl Plugin for RemotePlugin {
    fn init(
        &mut self,
        instance: &mut crate::instance::Instance,
        ext: Option<&mut crate::ext::ExtList>,
    ) -> crate::HyResult<()> {
        let mut remote_methods = RemoteMethods::new();

        if let Some(create_info) = ext.and_then(|ext| ext.get::<RemotePluginCreateInfo>()) {
            self.address = create_info.host;
            self.port = create_info.port;
        }

        for (name, handler) in self.methods.drain(..) {
            remote_methods.insert(
                name,
                match handler {
                    RemoteMethodHandler::Instant(system) => {
                        RemoteMethodSystemId::Instant(instance.world.register_boxed_system(system))
                    }
                    RemoteMethodHandler::Watching(system) => {
                        RemoteMethodSystemId::Watching(instance.world.register_boxed_system(system))
                    }
                },
            );
        }

        instance
            .world
            .get_resource_mut::<ScheduleOrder>()
            .unwrap()
            .main_labels
            .insert_after(Last, RemoteLast);

        instance
            .insert_resource(remote_methods)
            .init_resource::<schemas::SchemaTypesMetadata>()
            .init_resource::<RemoteWatchingRequests>()
            .add_systems(PreStartup, setup_mailbox_channel)
            .add_systems(
                RemoteLast,
                (
                    process_remote_requests,
                    process_ongoing_watching_requests,
                    remove_closed_watching_requests,
                )
                    .chain(),
            );

        instance
            .insert_resource(HostAddress(self.address))
            .insert_resource(HostPort(self.port))
            .insert_resource(HostHeaders(self.headers.clone()))
            .add_systems(Startup, http::start_http_server);

        Ok(())
    }

    fn is_public(&self) -> bool {
        true
    }
}

register_plugin!(RemotePlugin);

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RemoteLast;

#[derive(Debug, Clone)]
pub struct BrpMessage {
    /// The request method.
    pub method: String,

    /// The request params.
    pub params: Option<serde_json::Value>,

    /// The channel on which the response is to be sent.
    ///
    /// The value sent here is serialized and sent back to the client.
    pub sender: Sender<BrpResult>,
}

#[derive(Debug, Resource, Clone)]
pub struct Headers {
    headers: HashMap<HeaderName, HeaderValue>,
}

impl Headers {
    /// Create a new instance of `Headers`.
    pub fn new() -> Self {
        Self {
            headers: HashMap::default(),
        }
    }

    /// Insert a key value pair to the `Headers` instance.
    pub fn insert(
        mut self,
        name: impl TryInto<HeaderName>,
        value: impl TryInto<HeaderValue>,
    ) -> Self {
        let Ok(header_name) = name.try_into() else {
            panic!("Invalid header name")
        };
        let Ok(header_value) = value.try_into() else {
            panic!("Invalid header value")
        };
        self.headers.insert(header_name, header_value);
        self
    }
}

impl Default for Headers {
    fn default() -> Self {
        Self::new()
    }
}

/// A resource containing the headers that Bevy will include in its HTTP responses.
///
#[derive(Debug, Resource)]
struct HostHeaders(pub Headers);

/// Holds the [`BrpMessage`]'s of all ongoing watching requests along with their handlers.
#[derive(Debug, Resource, Default)]
pub struct RemoteWatchingRequests(Vec<(BrpMessage, RemoteWatchingMethodSystemId)>);

/// A resource holding the matching sender for the [`BrpReceiver`]'s receiver.
#[derive(Debug, Resource)]
pub struct BrpSender(Sender<BrpMessage>);

impl std::ops::Deref for BrpSender {
    type Target = Sender<BrpMessage>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for BrpSender {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A resource that receives messages sent by Bevy Remote Protocol clients.
///
/// Every frame, the `process_remote_requests` system drains this mailbox and
/// processes the messages within.
#[derive(Debug, Resource)]
pub struct BrpReceiver(Receiver<BrpMessage>);

impl std::ops::Deref for BrpReceiver {
    type Target = Receiver<BrpMessage>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for BrpReceiver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

const CHANNEL_SIZE: usize = 16;
fn setup_mailbox_channel(mut commands: Commands) {
    // Create the channel and the mailbox.
    let (request_sender, request_receiver) = async_channel::bounded(CHANNEL_SIZE);
    commands.insert_resource(BrpSender(request_sender));
    commands.insert_resource(BrpReceiver(request_receiver));
}

mod http {
    use std::{
        convert::Infallible,
        net::{TcpListener, TcpStream},
        pin::Pin,
        task::{Context, Poll},
    };

    use async_io::Async;
    use bevy_remote::{BrpBatch, BrpRequest, BrpResponse};
    use bevy_tasks::futures_lite::StreamExt;
    use http_body_util::{BodyExt, Full};
    use hyper::{
        Request, Response,
        body::{Body, Bytes, Frame, Incoming},
        header::HeaderValue,
        server::conn::http1,
        service,
    };
    use serde_json::Value;
    use smol_hyper::rt::{FuturesIo, SmolTimer};

    use crate::HyResult;

    use super::*;

    /// A system that starts up the Bevy Remote Protocol HTTP server.
    pub(super) fn start_http_server(
        logger: Res<LoggerStateRes>,
        request_sender: Res<BrpSender>,
        address: Res<HostAddress>,
        remote_port: Res<HostPort>,
        headers: Res<HostHeaders>,
    ) {
        hyinfo!(
            logger;
            "Starting Bevy Remote Protocol server on {}:{}",
            address.0,
            remote_port.0
        );

        IoTaskPool::get()
            .spawn(server_main(
                address.0,
                remote_port.0,
                request_sender.clone(),
                headers.0.clone(),
            ))
            .detach();
    }

    /// The Bevy Remote Protocol server main loop.
    async fn server_main(
        address: IpAddr,
        port: u16,
        request_sender: Sender<BrpMessage>,
        headers: Headers,
    ) -> HyResult<()> {
        listen(
            Async::<TcpListener>::bind((address, port))?,
            &request_sender,
            &headers,
        )
        .await
    }

    async fn listen(
        listener: Async<TcpListener>,
        request_sender: &Sender<BrpMessage>,
        headers: &Headers,
    ) -> HyResult<()> {
        loop {
            let (client, _) = listener.accept().await?;

            let request_sender = request_sender.clone();
            let headers = headers.clone();
            IoTaskPool::get()
                .spawn(async move {
                    let _ = handle_client(client, request_sender, headers).await;
                })
                .detach();
        }
    }

    async fn handle_client(
        client: Async<TcpStream>,
        request_sender: Sender<BrpMessage>,
        headers: Headers,
    ) -> HyResult<()> {
        http1::Builder::new()
            .timer(SmolTimer::new())
            .serve_connection(
                FuturesIo::new(client),
                service::service_fn(|request| {
                    process_request_batch(request, &request_sender, &headers)
                }),
            )
            .await?;

        Ok(())
    }

    /// A helper function for the Bevy Remote Protocol server that handles a batch
    /// of requests coming from a client.
    async fn process_request_batch(
        request: Request<Incoming>,
        request_sender: &Sender<BrpMessage>,
        headers: &Headers,
    ) -> HyResult<Response<BrpHttpBody>> {
        let batch_bytes = request.into_body().collect().await?.to_bytes();
        let batch: Result<BrpBatch, _> = serde_json::from_slice(&batch_bytes);

        let result = match batch {
            Ok(BrpBatch::Single(request)) => {
                let response = process_single_request(request, request_sender).await?;

                match response {
                    BrpHttpResponse::Complete(res) => {
                        BrpHttpResponse::Complete(serde_json::to_string(&res)?)
                    }
                    BrpHttpResponse::Stream(stream) => BrpHttpResponse::Stream(stream),
                }
            }
            Ok(BrpBatch::Batch(requests)) => {
                let mut responses = Vec::new();

                for request in requests {
                    let response = process_single_request(request.clone(), request_sender).await?;
                    match response {
                        BrpHttpResponse::Complete(res) => responses.push(res),
                        BrpHttpResponse::Stream(BrpStream { id, .. }) => {
                            responses.push(BrpResponse::new(
                                id,
                                Err(BrpError {
                                    code: error_codes::INVALID_REQUEST,
                                    message: "Streaming can not be used in batch requests"
                                        .to_string(),
                                    data: None,
                                }),
                            ));
                        }
                    }
                }

                BrpHttpResponse::Complete(serde_json::to_string(&responses)?)
            }
            Err(err) => {
                let err = BrpResponse::new(
                    None,
                    Err(BrpError {
                        code: error_codes::INVALID_REQUEST,
                        message: err.to_string(),
                        data: None,
                    }),
                );

                BrpHttpResponse::Complete(serde_json::to_string(&err)?)
            }
        };

        let mut response = match result {
            BrpHttpResponse::Complete(serialized) => {
                let mut response = Response::new(BrpHttpBody::Complete(Full::new(Bytes::from(
                    serialized.as_bytes().to_owned(),
                ))));
                response.headers_mut().insert(
                    hyper::header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                response
            }
            BrpHttpResponse::Stream(stream) => {
                let mut response = Response::new(BrpHttpBody::Stream(stream));
                response.headers_mut().insert(
                    hyper::header::CONTENT_TYPE,
                    HeaderValue::from_static("text/event-stream"),
                );
                response
            }
        };
        for (key, value) in &headers.headers {
            response.headers_mut().insert(key, value.clone());
        }
        Ok(response)
    }

    /// A helper function for the Bevy Remote Protocol server that processes a single
    /// request coming from a client.
    async fn process_single_request(
        request: Value,
        request_sender: &Sender<BrpMessage>,
    ) -> HyResult<BrpHttpResponse<BrpResponse, BrpStream>> {
        // Reach in and get the request ID early so that we can report it even when parsing fails.
        let id = request.as_object().and_then(|map| map.get("id")).cloned();

        let request: BrpRequest = match serde_json::from_value(request) {
            Ok(v) => v,
            Err(err) => {
                return Ok(BrpHttpResponse::Complete(BrpResponse::new(
                    id,
                    Err(BrpError {
                        code: error_codes::INVALID_REQUEST,
                        message: err.to_string(),
                        data: None,
                    }),
                )));
            }
        };

        if request.jsonrpc != "2.0" {
            return Ok(BrpHttpResponse::Complete(BrpResponse::new(
                id,
                Err(BrpError {
                    code: error_codes::INVALID_REQUEST,
                    message: String::from("JSON-RPC request requires `\"jsonrpc\": \"2.0\"`"),
                    data: None,
                }),
            )));
        }

        let watch = request.method.contains("+watch");
        let size = if watch { 8 } else { 1 };
        let (result_sender, result_receiver) = async_channel::bounded(size);

        let _ = request_sender
            .send(BrpMessage {
                method: request.method.clone(),
                params: request.params.clone(),
                sender: result_sender,
            })
            .await;

        if watch {
            Ok(BrpHttpResponse::Stream(BrpStream {
                id: request.id,
                rx: Box::pin(result_receiver),
                is_disconnected: false,
            }))
        } else {
            let result = match result_receiver.recv().await {
                Ok(res) => res,
                Err(err) => {
                    return Err(err.into());
                }
            };

            Ok(BrpHttpResponse::Complete(BrpResponse::new(
                request.id, result,
            )))
        }
    }

    struct BrpStream {
        id: Option<Value>,
        rx: Pin<Box<Receiver<BrpResult>>>,
        is_disconnected: bool,
    }

    impl Body for BrpStream {
        type Data = Bytes;
        type Error = Infallible;

        fn poll_frame(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
            match self.as_mut().rx.poll_next(cx) {
                Poll::Ready(result) => match result {
                    Some(result) => {
                        let response = BrpResponse::new(self.id.clone(), result);
                        let serialized = serde_json::to_string(&response).unwrap();
                        let bytes =
                            Bytes::from(format!("data: {serialized}\n\n").as_bytes().to_owned());
                        let frame = Frame::data(bytes);
                        Poll::Ready(Some(Ok(frame)))
                    }
                    None => Poll::Ready(None),
                },
                Poll::Pending => Poll::Pending,
            }
        }

        fn is_end_stream(&self) -> bool {
            self.is_disconnected
        }
    }

    enum BrpHttpResponse<C, S> {
        Complete(C),
        Stream(S),
    }

    enum BrpHttpBody {
        Complete(Full<Bytes>),
        Stream(BrpStream),
    }

    impl Body for BrpHttpBody {
        type Data = Bytes;
        type Error = Infallible;

        fn poll_frame(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
            match &mut *self.get_mut() {
                BrpHttpBody::Complete(body) => Body::poll_frame(Pin::new(body), cx),
                BrpHttpBody::Stream(body) => Body::poll_frame(Pin::new(body), cx),
            }
        }
    }
}

/// A system that receives requests placed in the [`BrpReceiver`] and processes
/// them, using the [`RemoteMethods`] resource to map each request to its handler.
///
/// This needs exclusive access to the [`World`] because clients can manipulate
/// anything in the ECS.
fn process_remote_requests(world: &mut World) {
    if !world.contains_resource::<BrpReceiver>() {
        return;
    }

    while let Ok(message) = world.resource_mut::<BrpReceiver>().try_recv() {
        // Fetch the handler for the method. If there's no such handler
        // registered, return an error.
        let Some(&handler) = world.resource::<RemoteMethods>().get(&message.method) else {
            hyerror!(
                world;
                "Received request for unknown method `{}`. Returning error.",
                message.method
            );
            let _ = message.sender.send(Err(BrpError {
                code: error_codes::METHOD_NOT_FOUND,
                message: format!("Method `{}` not found", message.method),
                data: None,
            }));
            return;
        };

        match handler {
            RemoteMethodSystemId::Instant(id) => {
                hydebug!(
                    world;
                    "Received request for method `{}`. Running handler.",
                    message.method
                );

                let result = match world.run_system_with(id, message.params) {
                    Ok(result) => result,
                    Err(error) => {
                        hyerror!(
                            world;
                            "Failed to run handler for method `{}`: {error}",
                            message.method
                        );
                        let _ = message.sender.send(Err(BrpError {
                            code: error_codes::INTERNAL_ERROR,
                            message: format!("Failed to run method handler: {error}"),
                            data: None,
                        }));
                        continue;
                    }
                };

                let _ = message.sender.send_blocking(result);
            }
            RemoteMethodSystemId::Watching(id) => {
                hydebug!(
                    world;
                    "Received request for watching method `{}`. Registering watching request.",
                    message.method
                );

                world
                    .resource_mut::<RemoteWatchingRequests>()
                    .0
                    .push((message, id));
            }
        }
    }
}

/// A system that checks all ongoing watching requests for changes that should be sent
/// and handles it if so.
fn process_ongoing_watching_requests(world: &mut World) {
    world.resource_scope::<RemoteWatchingRequests, ()>(|world, requests| {
        for (message, system_id) in requests.0.iter() {
            let handler_result = process_single_ongoing_watching_request(world, message, system_id);
            let sender_result = match handler_result {
                Ok(Some(value)) => message.sender.try_send(Ok(value)),
                Err(err) => message.sender.try_send(Err(err)),
                Ok(None) => continue,
            };

            if sender_result.is_err() {
                hydebug!(
                    world;
                    "Failed to send response for watching method `{}`. Closing sender.",
                    message.method
                );

                // The [`remove_closed_watching_requests`] system will clean this up.
                message.sender.close();
            }
        }
    });
}

fn process_single_ongoing_watching_request(
    world: &mut World,
    message: &BrpMessage,
    system_id: &RemoteWatchingMethodSystemId,
) -> BrpResult<Option<serde_json::Value>> {
    world
        .run_system_with(*system_id, message.params.clone())
        .map_err(|error| BrpError {
            code: error_codes::INTERNAL_ERROR,
            message: format!("Failed to run method handler: {error}"),
            data: None,
        })?
}

fn remove_closed_watching_requests(
    mut requests: ResMut<RemoteWatchingRequests>,
    logger: Res<LoggerStateRes>,
) {
    for i in (0..requests.0.len()).rev() {
        let Some((message, _)) = requests.0.get(i) else {
            unreachable!()
        };

        if message.sender.is_closed() {
            hydebug!(
                logger;
                "Removing closed watching request for method `{}`.",
                message.method
            );
            requests.0.swap_remove(i);
        }
    }
}
