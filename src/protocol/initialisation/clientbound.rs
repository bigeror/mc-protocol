use crate::datatypes::{StringBuffer, VarInt};
use json::{object, stringify};

fn add_length (packet: Vec<u8>) -> Vec<u8> {
    if let Ok(length) = VarInt::encode(packet.len() as u64) 
        { return vec![length, packet].concat()}
    packet
}

pub struct ClientBoundPackets {
    pub status: StatusClientBound
}

pub struct StatusClientBound {
    pub status_response: fn() -> Vec<u8>,
    pub ping_response: fn(value: &Vec<u8>) -> Vec<u8>
}

pub static CLIENT_BOUND_PACKETS: ClientBoundPackets = ClientBoundPackets {
    status: StatusClientBound { 
        status_response: || {
            let status = object!{ version: {
                    name: "1.21.8",
                    protocol: 772
                },
                players: {
                    max: -1,
                    online: 0,
                    sample: []
                },
                description: {
                    text: ":)"
                },
                enforcesSecureChat: false
            };
            let status_text = StringBuffer::encode(&stringify(status)).unwrap();
            add_length([vec![0], Vec::from(status_text)].concat())
        }, 
        ping_response: |packet| add_length([vec![1], packet[1..9].to_vec()].concat()), 
    }
};