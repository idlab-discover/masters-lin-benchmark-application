wit_bindgen::generate!({ generate_all });

use exports::wasi::http::incoming_handler::Guest;
use wasi::http::types::*;
// use std::time::{SystemTime, UNIX_EPOCH};
use wasi::clocks::monotonic_clock;
use wasi::io::streams::StreamError;
use anyhow::{anyhow, bail, ensure, Result};

struct HttpServer;

/// Maximum bytes to read at a time from the incoming request body
/// this value is chosen somewhat arbitrarily, and is not a limit for bytes read,
/// but is instead the amount of bytes to be read *at once*
const MAX_READ_BYTES: u32 = 2048;

/// Maximum bytes to write at a time, due to the limitations on wasi-io's blocking_write_and_flush()
const MAX_WRITE_BYTES: usize = 4096;

// Pre-allocate a large buffer to ensure same startup memory usage
const MAX_SIZE: usize = 1_000;
static DUMMY_BUFFER: [u8; MAX_SIZE] = [0; MAX_SIZE];

impl Guest for HttpServer {
    fn handle(_request: IncomingRequest, response_out: ResponseOutparam) { //request not needed
        // 1) Attempt to read the EFFECTIVE_SIZE env var. Return 400 if missing or invalid.
        let requested_size = match get_effective_size() {
            Ok(s) => s,
            Err(msg) => {
                eprintln!("DEBUG: get_effective_size() => Err({msg})");
                return http_error(response_out, 400, &msg);
            }
        };

        eprintln!("DEBUG: requested_size = {requested_size}, MAX_SIZE = {MAX_SIZE}");

        // 2) If requested_size > MAX_SIZE, return a 400 with explanation
        if requested_size > MAX_SIZE {
            eprintln!("DEBUG: requested_size > MAX_SIZE => 400 error.");
            let msg = format!("Requested size={requested_size} exceeds MAX_SIZE={MAX_SIZE}");
            return http_error(response_out, 400, &msg);
        }

        // 3) Slice the static buffer and convert to a string
        let payload_slice = &DUMMY_BUFFER[..requested_size];
        let payload = String::from_utf8_lossy(payload_slice);

        eprintln!("DEBUG: Sliced buffer of size {requested_size}, continuing with 200 OK.");




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
        
       // Prepare the response text
        let response_text = format!(
            "Hello! I got pong {pong}\n\
             Start timestamp: {start_time}\n\
             End timestamp: {end_time}\n\
             Elapsed time(ns): {elapsed_time_ns}\n"
        );

        // Use the `send_body` method, which writes + finishes automatically
        response
            .send_body(response_text.as_bytes())
            .expect("failed to send response body");

        // Return the final response to the caller
        ResponseOutparam::set(response_out, Ok(response));
    }
}

//----------------------------------
// Env var reading function
//----------------------------------
fn get_effective_size() -> std::result::Result<usize, String> {
    // We want *no default*, so we error out if it's missing
    let var_result = std::env::var("EFFECTIVE_SIZE");
    eprintln!("DEBUG: Attempting to read env var EFFECTIVE_SIZE => {:?}", var_result);

    match var_result {
        // If present, parse it
        Ok(s) => match s.parse::<usize>() {
            Ok(val) => {
                eprintln!("DEBUG: Parsed EFFECTIVE_SIZE={val}");
                Ok(val)
            }
            Err(e) => {
                let msg = format!("Invalid integer in EFFECTIVE_SIZE='{s}': {e}");
                eprintln!("DEBUG: {msg}");
                Err(msg)
            }
        },
        // If missing, that's an error
        Err(e) => {
            let msg = format!(
                "EFFECTIVE_SIZE is missing in environment: {e}.\n\
                 Please set EFFECTIVE_SIZE before running the actor."
            );
            eprintln!("DEBUG: {msg}");
            Err(msg)
        }
    }
}

//----------------------------------
// Simple helper for 4xx responses
//----------------------------------
fn http_error(response_out: ResponseOutparam, code: u16, msg: &str) {
    let err_resp = OutgoingResponse::new(Fields::new());
    err_resp.set_status_code(code).unwrap();
    // We don't want to panic if sending body fails, so we ignore any error
    let _ = err_resp.send_body(msg.as_bytes());
    ResponseOutparam::set(response_out, Ok(err_resp));
}



// NOTE: Since wit-bindgen makes `IncomingRequest` available to us as a local type,
// we can add convenience functions to it
impl IncomingRequest {
    /// This is a convenience function that writes out the body of a IncomingRequest (from wasi:http)
    /// into anything that supports [`std::io::Write`]
    fn read_body(self) -> Result<Vec<u8>> {
        // Read the body
        let incoming_req_body = self
            .consume()
            .map_err(|()| anyhow!("failed to consume incoming request body"))?;
        let incoming_req_body_stream = incoming_req_body
            .stream()
            .map_err(|()| anyhow!("failed to build stream for incoming request body"))?;
        let mut buf = Vec::<u8>::with_capacity(MAX_READ_BYTES as usize);
        loop {
            match incoming_req_body_stream.read(MAX_READ_BYTES as u64) {
                Ok(bytes) if bytes.is_empty() => break,
                Ok(bytes) => {
                    ensure!(
                        bytes.len() <= MAX_READ_BYTES as usize,
                        "read more bytes than requested"
                    );
                    buf.extend(bytes);
                }
                Err(StreamError::Closed) => break,
                Err(e) => bail!("failed to read bytes: {e}"),
            }
        }
        buf.shrink_to_fit();
        drop(incoming_req_body_stream);
        IncomingBody::finish(incoming_req_body);
        Ok(buf)
    }
}

// NOTE: Since wit-bindgen makes `OutgoingBody` available to us as a local type,
// we can add convenience functions to it
impl OutgoingResponse {
    /// This is a convenience function that writes out the body of a IncomingRequest (from wasi:http)
    /// into anything that supports [`std::io::Read`]
    fn send_body(&self, buf: &[u8]) -> Result<()> {
        let body = self.body().expect("failed to open outgoing response body");
        let out = body
            .write()
            .expect("failed to start writing to outgoing response body");
        for chunk in buf.chunks(MAX_WRITE_BYTES) {
            out.blocking_write_and_flush(chunk)
                .map_err(|e| anyhow!("failed to write chunk: {e}"))?;
        }
        drop(out);
        OutgoingBody::finish(body, None).unwrap();
        Ok(())
    }
}

export!(HttpServer);
