use zmq::{Context, Socket, Message};
use std::io::{self, Write};
use std::thread::*;
use std::time::Duration;

pub struct ZmqClient {
    requester: Socket,
    msg: Message,
}

impl ZmqClient {
    pub fn new() -> Self {
        let context = Context::new();
        let requester = context.socket(zmq::REQ).unwrap();
        // let s = format!("ipc://zmq_{name}");
        let s = format!("tcp://localhost:5555");
        requester.connect(&s).expect("Failed to connect to {s}");
        requester.set_sndtimeo(1000).unwrap();
        requester.set_rcvtimeo(1000).unwrap();
        let mut msg = Message::new();
        requester.send(vec![42], 0).unwrap();
        requester.recv(&mut msg, 0).expect("Failed to receive ZMQ message within timeout");
        assert_eq!(msg.as_str().unwrap(), "ackINIT");
        Self { requester, msg, }
    }


    pub fn send_img(&mut self, buffer: &Vec<u8>) {
        assert!(self.requester.connect("tcp://localhost:5555").is_ok());
        // let img = vec![32u8; 512];
        println!("Sending 8-bit Image...");
        self.requester.send(buffer, 0).unwrap();
        self.requester.recv(&mut self.msg, 0).unwrap();
        assert_eq!(self.msg.as_str().unwrap(), "ack");
    }
}