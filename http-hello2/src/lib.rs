wit_bindgen::generate!({ generate_all });

use exports::wasi::http::incoming_handler::Guest;
use wasi::http::types::*;
use wasi::clocks::monotonic_clock;

/// Maximum bytes to write at once with `blocking_write_and_flush()`
const MAX_WRITE_BYTES: usize = 4096;

/// Total static buffer size (fixed memory usage)
const MAX_SIZE_BIG_BYTES: usize = 625_000_000;
static BIG_BYTES: [u8; MAX_SIZE_BIG_BYTES] = [0; MAX_SIZE_BIG_BYTES];

/// Required payload size for the current benchmark build
const REQUIRED_SIZE: usize = 256_000;

struct HttpServer;

impl Guest for HttpServer {
    fn handle(_request: IncomingRequest, response_out: ResponseOutparam) {
        // Get the fixed-size payload slice from the preallocated buffer
        let slice = &BIG_BYTES[..REQUIRED_SIZE];
        let big_string = String::from_utf8_lossy(slice).to_string();
        
        let start_ns = monotonic_clock::now();
        let pong = example::pong::pingpong::ping(&big_string);
        let end_ns = monotonic_clock::now();

        let payload_len = big_string.len();
        let pong_len = pong.len();
        let response_text = format!(
            "Start timestamp: {start_ns}\n\
             End timestamp:   {end_ns}\n\
             Payload size:    {payload_len}\n\
             Pong size:       {pong_len}\n"
        );

        // 1) Create the OutgoingResponse
        let response = OutgoingResponse::new(Fields::new());
        response.set_status_code(200).unwrap();

        // 2) Get the response body
        let body = response.body().expect("failed to open response body");
        
        // 3) Set ResponseOutparam      
        ResponseOutparam::set(response_out, Ok(response));

        // 4) Get the output stream
        let out_stream = body
            .write()
            .expect("failed to acquire output-stream handle for response");

        // 5) Write the response in a loop
        // Write in small chunks to avoid exceeding buffer limits
        for chunk in response_text.as_bytes().chunks(MAX_WRITE_BYTES) {
            out_stream
                .blocking_write_and_flush(chunk)
                .expect("failed to write chunk to response");
        }

        // 6) Drop the stream
        drop(out_stream);
        
        // 7) Finish the outgoing body
        OutgoingBody::finish(body, None).expect("failed to finish response body");
    }
}

export!(HttpServer);

