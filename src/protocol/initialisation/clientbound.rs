use crate::datatypes::StringBuffer;
use json::{object, stringify};

pub struct ClientBoundPackets {
    pub status: StatusClientBound
}

pub struct StatusClientBound {
    pub status_response: fn() -> Vec<u8>,
    pub ping_response: fn(value: i64) -> Vec<u8>
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
                    text: ":D"
                },
                enforcesSecureChat: false
            };
            let status_text = StringBuffer::encode(&stringify(status)).unwrap();
            [vec![0], Vec::from(status_text)].concat()
        }, 
        ping_response: |num| [vec![1], num.to_be_bytes().to_vec()].concat(), 
    }
};