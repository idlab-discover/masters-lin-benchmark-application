wit_bindgen::generate!({ generate_all });


use exports::wasi::http::incoming_handler::Guest;
use wasi::http::types::*;
// use std::time::{SystemTime, UNIX_EPOCH};
use wasi::clocks::monotonic_clock;

struct HttpServer;

impl Guest for HttpServer {
    fn handle(_request: IncomingRequest, response_out: ResponseOutparam) { //request not needed
        let payload = "This is a test payload".to_string();
        
        // let start_timestamp = SystemTime::now()
        //     .duration_since(UNIX_EPOCH)
        //     .expect("Time went backwards")
        //     .as_micros();
        let start_time = monotonic_clock::now();

        let pong = example::pong::pingpong::ping(&payload);
        
        let end_time = monotonic_clock::now();

        // Calculate elapsed time in nanoseconds
        let elapsed_time_ns = end_time - start_time;
        
        // let end_timestamp = SystemTime::now()
        //     .duration_since(UNIX_EPOCH)
        //     .expect("Time went backwards")
        //     .as_micros();
            
        let response = OutgoingResponse::new(Fields::new());
        response.set_status_code(200).unwrap(); 
        let response_body = response.body().unwrap();

        response_body
            .write()
            .unwrap()
            .blocking_write_and_flush(format!("Hello! I got pong {pong}
            Start timestamp: {start_time}
            End timestamp: {end_time}
            Elapsed time(ns): {elapsed_time_ns}").as_bytes())
            .unwrap();
        OutgoingBody::finish(response_body, None).expect("failed to finish response body");
        ResponseOutparam::set(response_out, Ok(response));
    }
}

export!(HttpServer);
