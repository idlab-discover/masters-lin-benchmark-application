wit_bindgen::generate!({ generate_all });

use exports::wasi::http::incoming_handler::Guest;
use wasi::http::types::*;

const MAX_SIZE_BIG_BYTES: usize = 80*1024*1024; // Max with pooling allocator: ~80*1024*1024

#[used]
#[no_mangle]
static BIG_BYTES: [u8; MAX_SIZE_BIG_BYTES] = [0; MAX_SIZE_BIG_BYTES];
const REQUIRED_SIZE: usize = 80*1024*1024;

struct HttpServer;

impl Guest for HttpServer {
    fn handle(_request: IncomingRequest, response_out: ResponseOutparam) {
        // Get the fixed-size payload slice from the preallocated buffer
        let slice = &BIG_BYTES[..REQUIRED_SIZE];
        let big_string = String::from_utf8_lossy(slice).to_string();
        
        let _pong = example::pong::pingpong::ping(&big_string);

        // Build the HTTP response
        let response = OutgoingResponse::new(Fields::new());
        response.set_status_code(200).unwrap();

        // Open body and finish it
        let body = response.body().expect("failed to open response body");
        OutgoingBody::finish(body, None).expect("failed to finish response body");

        //Return the response with empty body ---
        ResponseOutparam::set(response_out, Ok(response));
    }
}

export!(HttpServer);

