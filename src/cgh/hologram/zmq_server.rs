use zmq::{Context, Socket, Message};
use std::io::{self, Write};
use std::thread::*;
use std::time::Duration;
use serde_json;
use serde::{Serialize, Deserialize};

pub struct ZmqServer {
    responder: zmq::Socket,
}

#[derive(Serialize, Deserialize)]
pub struct Phmask {
    pub mask: Vec<u8>,
}

impl ZmqServer {
    pub fn new() -> Self {
        let context = Context::new();
        let responder = context.socket(zmq::REP).unwrap();
        let s = format!("tcp://*:5555");
        responder.bind(&s).unwrap();
        responder.set_sndtimeo(10000).unwrap();
        responder.set_rcvtimeo(10000).unwrap();
        let mut msg: Message = Message::new();
        loop { 
            responder.recv(&mut msg, 0);
            if msg.as_str().unwrap() == "INIT" {     
                println!("Received INIT from client", );
                responder.send("ackINIT", 0).unwrap();
                break;
            } else {
                sleep(Duration::from_millis(50));
            }
        }
        Self { responder, }
    }

    pub fn send_img(&mut self, buffer: &Vec<u8>) {
        let mut msg: Message = Message::new();
        // let phmask = vec![0u8; 1920*1152];   
        // let phmask = Phmask {mask: vec![15u8; 10000],};   
        // let serialized = serde_json::to_vec(&phmask).unwrap();
        self.responder.recv(&mut msg, 0).unwrap();
        assert_eq!(msg.as_str().unwrap(), "xfer");
        println!("Sending SLM phase mask...");
        self.responder.send(buffer, 0).unwrap();
    }
}