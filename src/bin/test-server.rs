extern crate tungstenite;

use std::thread::spawn;
use std::net::TcpListener;

use tungstenite::accept_hdr;
use tungstenite::handshake::server::Request;

fn main() {
    let server = TcpListener::bind("127.0.0.1:8080").unwrap();
    for stream in server.incoming() {
        spawn(move || {
            let callback = |req: &Request| {
                println!("Received a new ws handshake");
                println!("The request's path is: {}", req.path);
                println!("The request's headers are:");
                for &(ref header, _ /* value */) in req.headers.iter() {
                    println!("* {}", header);
                }

                // Let's add an additional header to our response to the client.
                let extra_headers = vec![
                    (String::from("MyCustomHeader"), String::from(":)")),
                    (String::from("SOME_TUNGSTENITE_HEADER"), String::from("header_value")),
                ];
                Ok(Some(extra_headers))
            };
            let mut websocket = accept_hdr(stream.unwrap(), callback).unwrap();
            let msg: &[u8] = b"4204204200000000e4ef6f9a59000000020000000000000000000000000000080808088f30a4d938d4130d1a3396dede1505c72b7f75ac9f9b80d1ad7e368b39f3b10500e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100bfafd3f1cc800309cb931081257b4e7edb0f9ea400187c550fd9141447ea086a5324dcf027dd4a30a932c441f365a25e86b173defa4b8e58948253471b81b72cf0000000000000000000000000000000000000000000000000000000000000000";
            let msg = tungstenite::Message::binary(msg);
            websocket.write_message(msg).unwrap();

            loop {
                let msg = websocket.read_message().unwrap();
                if msg.is_binary() || msg.is_text() {
                    websocket.write_message(msg).unwrap();
                }
            }
        });
    }
}