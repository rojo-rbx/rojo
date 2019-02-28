// TODO: This module needs to be public for visualize, we should move
// PublicInstanceMetadata and switch this private!
pub mod api;
mod interface;

use std::{
    collections::HashSet,
    thread,
    sync::{Arc, Mutex},
};

use log::trace;
use futures::{
    future::{self, FutureResult},
    sync::oneshot,
    Future,
};
use hyper::{
    service::Service,
    Body,
    Request,
    Response,
    Server,
};

use crate::{
    session_id::SessionId,
    message_queue::MessageQueue,
    rbx_session::RbxSession,
    imfs::Imfs,
    snapshot_reconciler::InstanceChanges,
};

use self::{
    api::ApiService,
    interface::InterfaceService,
};

#[derive(Clone)]
pub struct ServiceDependencies {
    pub session_id: SessionId,
    pub serve_place_ids: Option<HashSet<u64>>,
    pub message_queue: Arc<MessageQueue<InstanceChanges>>,
    pub rbx_session: Arc<Mutex<RbxSession>>,
    pub imfs: Arc<Mutex<Imfs>>,
}

pub struct RootService {
    api: api::ApiService,
    interface: interface::InterfaceService,
}

impl Service for RootService {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<dyn Future<Item = Response<Self::ReqBody>, Error = Self::Error> + Send>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        trace!("{} {}", request.method(), request.uri().path());

        if request.uri().path().starts_with("/api") {
            self.api.call(request)
        } else {
            self.interface.call(request)
        }
    }
}

impl RootService {
    pub fn new(dependencies: ServiceDependencies) -> RootService {
        RootService {
            api: ApiService::new(dependencies.clone()),
            interface: InterfaceService::new(dependencies.clone()),
        }
    }
}

pub struct LiveServer {
    shutdown_tx: oneshot::Sender<()>,
    finished_rx: oneshot::Receiver<()>,
}

impl LiveServer {
    pub fn start(dependencies: ServiceDependencies, port: u16) -> LiveServer {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (finished_tx, finished_rx) = oneshot::channel();

        let address = ([127, 0, 0, 1], port).into();

        let server = Server::bind(&address)
            .serve(move || {
                let service: FutureResult<_, hyper::Error> =
                    future::ok(RootService::new(dependencies.clone()));
                service
            })
            .with_graceful_shutdown(shutdown_rx)
            .map_err(|e| eprintln!("Server error: {}", e));

        thread::spawn(move || {
            hyper::rt::run(server);
            let _dont_care = finished_tx.send(());
        });

        LiveServer {
            shutdown_tx,
            finished_rx,
        }
    }

    pub fn stop(self) {
        let _dont_care = self.shutdown_tx.send(());
        self.finished_rx.wait();
    }
}