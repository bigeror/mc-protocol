use std::{collections::HashMap, pin::Pin, sync::{Arc, LazyLock}};
use tokio::{io::AsyncWriteExt};

use crate::protocol::{ProtocolHandler, RuntimeError, initialisation::clientbound::CLIENT_BOUND_PACKETS};

type BoxedFuture<'a> = Pin<Box<dyn Future<Output = Option<RuntimeError>> + Send + 'a>>;
pub type Responses = HashMap<u8, Arc<dyn for<'a> Fn(&'a Vec<u8>, &'a mut ProtocolHandler) -> BoxedFuture<'a> + Send + Sync + 'static>>;

pub struct ServerBoundPackets {
    pub status: Responses,
    pub login: Responses,
    pub configuration: Responses,
    pub play: Responses,
}

impl ServerBoundPackets {
    fn new() -> Self {
        ServerBoundPackets { 
            status: status_responses(), 
            login: HashMap::new(), 
            configuration: HashMap::new(), 
            play: HashMap::new() 
        }
    }
}

macro_rules! formalise {
    (|$arg1:pat_param, $arg2:pat_param| $body:block) => {Arc::new(|_packet, _handler| {
        Box::pin((async |$arg1 : &Vec<u8>, $arg2 : &mut ProtocolHandler| $body)(_packet, _handler)) as BoxedFuture
    })};
}

pub static SERVER_BOUND_PACKETS_INSTANCE: LazyLock<ServerBoundPackets> = LazyLock::new(ServerBoundPackets::new);

fn status_responses() -> Responses {
    let mut responses: Responses = HashMap::new();

    responses.insert(0x00, formalise!(|packet, handler| {
        let mut response = (CLIENT_BOUND_PACKETS.status.status_response)();
        _ = handler.writer.write_all(&mut response).await;
        None
    }));

    responses.insert(0x01, formalise!(|packet, handler| {
        let mut response = (CLIENT_BOUND_PACKETS.status.ping_response)(packet);
        _ = handler.writer.write_all(&mut response).await;
        None
    }));

    responses
}
