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

const MAX_SIZE: usize = 10_000;
static DUMMY_BUFFER: [u8; MAX_SIZE] = [0; MAX_SIZE];

impl Guest for HttpServer {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) { //request not needed
        // 2) Parse the request body for "size=..."
        let raw_body = match request.read_body() {
            Ok(b) => b,
            Err(_) => {
                let err = OutgoingResponse::new(Fields::new());
                err.set_status_code(400).unwrap();
                err.send_body(b"Failed to read request body").unwrap();
                ResponseOutparam::set(response_out, Ok(err));
                return;
            }
        };

        let body_str = String::from_utf8_lossy(&raw_body);
        let requested_size = match parse_size(&body_str) {
            Ok(s) => s,
            Err(e) => {
                let err = OutgoingResponse::new(Fields::new());
                err.set_status_code(400).unwrap();
                err.send_body(e.as_bytes()).unwrap();
                ResponseOutparam::set(response_out, Ok(err));
                return;
            }
        };

        if requested_size > MAX_SIZE {
            let err = OutgoingResponse::new(Fields::new());
            err.set_status_code(400).unwrap();
            let msg = format!("Requested size {requested_size} > MAX_SIZE {MAX_SIZE}");
            err.send_body(msg.as_bytes()).unwrap();
            ResponseOutparam::set(response_out, Ok(err));
            return;
        }

        // 4) Slice the big static buffer
        let payload_slice = &DUMMY_BUFFER[..requested_size];
        let payload = String::from_utf8_lossy(payload_slice);


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
        
        let response_text = format!(
            "Hello! I got pong {pong}\n\
             Start timestamp: {start_time}\n\
             End timestamp: {end_time}\n\
             Elapsed time(ns): {elapsed_time_ns}\n"
        );

        response
            .send_body(response_text.as_bytes())
            .expect("failed to send response body");

        ResponseOutparam::set(response_out, Ok(response));
    }
}

/// Expect the body to contain e.g. "size=2000000".
fn parse_size(body_str: &str) -> std::result::Result<usize, String> {
    if let Some(idx) = body_str.find("size=") {
        let after = &body_str[idx+5..];
        let trimmed = after.trim();
        match trimmed.parse::<usize>() {
            Ok(v) => Ok(v),
            Err(e) => Err(format!("Failed to parse integer after size=, got '{trimmed}': {e}")),
        }
    } else {
        Err(format!("Body must contain 'size=...' but got '{body_str}'"))
    }
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
