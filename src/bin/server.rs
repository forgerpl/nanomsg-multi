extern crate colored_logger;
extern crate flexi_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate nanomsg_multi_server;
extern crate tokio_core;

use tokio_core::reactor::Core;
use nanomsg_multi_server::{MultiServer, MultiServerFutures};
use nanomsg_multi_server::proto::PeerReply;
use nanomsg_multi_server::config::{GcInterval, MainSocketUrl, SessionTimeout};

use futures::{Future, Stream};

fn main() {
    flexi_logger::Logger::with_env()
        .format(colored_logger::formatter)
        .start()
        .expect("Logger initialization failed");

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    info!("Starting socket server");

    let server = MultiServer::new(
        MainSocketUrl::default(),
        SessionTimeout::default(),
        GcInterval::default(),
        handle.clone(),
    );

    let MultiServerFutures { server, sessions } = server.into_futures().expect("server error");

    handle.spawn(sessions.for_each(|session| {
        info!("Incoming new session {:?}", session);

        let (writer, reader) = session.connection.split();

        reader
            .map(|msg| {
                info!("Incoming message {:?}", msg);

                PeerReply::Response(1, Ok(None))
            })
            .forward(writer)
            .map(|_| ())
            .map_err(|_| ())
    }));

    core.run(server).expect("unable to run io loop");
}
