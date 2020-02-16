use log::*;

use colored_logger::FormatterBuilder;
use nanomsg_multi_server::config::{
    default_client_socket_url, GcInterval, MainSocketUrl, SessionTimeout,
};
use nanomsg_multi_server::proto::PeerReply;
use nanomsg_multi_server::{MultiServer, MultiServerFutures};
use tokio_core::reactor::Core;

use futures::{Future, Stream};

fn main() {
    let formatter = FormatterBuilder::default().build();
    flexi_logger::Logger::with_env()
        .format(formatter)
        .start()
        .expect("Logger initialization failed");

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    info!("Starting socket server");

    let server = MultiServer::new(
        MainSocketUrl::default(),
        SessionTimeout::default(),
        GcInterval::default(),
        default_client_socket_url,
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
