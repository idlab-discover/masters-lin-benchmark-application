wit_bindgen::generate!({ generate_all });

use exports::wasi::http::incoming_handler::Guest;
use wasi::http::types::*;
use wasi::clocks::monotonic_clock;

/// Maximum bytes to write at once with `blocking_write_and_flush()`
const MAX_WRITE_BYTES: usize = 4096;

// ---------------------------------------------------------
// Comments are the results with certain resource limits
// ---------------------------------------------------------
const MAX_SIZE_LARGE_STRING: usize = 2_147_483_000; // When composed ~<=2147483647 build ok, run not ok with almost immediate oom message; can run at ~1_500_000_000; run not ok with value in between fail(timeout) with no error message and host stopped responding

//const MAX_SIZE_BIG_BYTES: usize = 625_000_000;
//static BIG_BYTES: [u8; MAX_SIZE_BIG_BYTES] = [0; MAX_SIZE_BIG_BYTES]; // When composed 937_500_000 build ok, 968_750_000 build not ok for [0; MAX_SIZE_BIG_BYTES]; can run at ~625_000_000
//const MAX_SIZE_DUMYY_BUFFER: usize = 968_750_000; // When composed 937_500_000 build ok, 968_750_000 build not ok for [0; MAX_SIZE_DUMMY_BUFFER]
//static mut DUMMY_BUFFER: [u8; MAX_SIZE_DUMMY_BUFFER] = [0; MAX_SIZE_DUMMY_BUFFER];

struct HttpServer;

impl Guest for HttpServer {
    fn handle(_request: IncomingRequest, response_out: ResponseOutparam) {
        // Build a large payload and call `ping`
        let mut large_string = String::with_capacity(MAX_SIZE_LARGE_STRING); //2147483647
        large_string.extend((0..MAX_SIZE_LARGE_STRING).map(|_| '0'));
        
        // let big_string = String::from_utf8_lossy(&BIG_BYTES).to_string();
        
        // let slice = unsafe { &DUMMY_BUFFER[..MAX_SIZE_DUMMY_BUFFER] };
        // let big_string = String::from_utf8_lossy(slice);

        // Measure time around calling `ping`
        let start_ns = monotonic_clock::now();
        let pong = example::pong::pingpong::ping(&large_string);
        //let pong = example::pong::pingpong::ping(&big_string);
        let end_ns = monotonic_clock::now();


        // Prepare a final text response
        let payload_len = large_string.len();
        //let payload_len = big_string.len();
        let pong_len = pong.len();
        let response_text = format!(
            "Start timestamp: {start_ns}\n\
             End timestamp:   {end_ns}\n\
             Payload size:    {payload_len}\n\
             Pong size:       {pong_len}\n"
        );

        // ---------------------------------------------------------
        // 1) Create the OutgoingResponse
        // ---------------------------------------------------------
        let response = OutgoingResponse::new(Fields::new());
        response.set_status_code(200).unwrap();

        // ---------------------------------------------------------
        // 2) Get the body, then 3) set ResponseOutparam
        // ---------------------------------------------------------
        let body = response.body().expect("failed to open response body");
        ResponseOutparam::set(response_out, Ok(response));

        // ---------------------------------------------------------
        // 4) Get the output stream, 5) write in a loop
        // ---------------------------------------------------------
        let out_stream = body
            .write()
            .expect("failed to acquire output-stream handle for response");

        // Write in small chunks to avoid exceeding buffer limits
        for chunk in response_text.as_bytes().chunks(MAX_WRITE_BYTES) {
            out_stream
                .blocking_write_and_flush(chunk)
                .expect("failed to write chunk to response");
        }

        // ---------------------------------------------------------
        // 6) Drop the stream, 7) finish the outgoing body
        // ---------------------------------------------------------
        drop(out_stream);
        OutgoingBody::finish(body, None).expect("failed to finish response body");
    }
}

export!(HttpServer);

