wit_bindgen::generate!({ generate_all });

use exports::wasi::http::incoming_handler::Guest;
use wasi::http::types::*;
// use std::time::{SystemTime, UNIX_EPOCH};
use wasi::clocks::monotonic_clock;
use wasi::io::streams::StreamError;
use anyhow::{anyhow, bail, ensure, Result};
use once_cell::sync::OnceCell;

struct HttpServer;

/// Maximum bytes to read at a time from the incoming request body
/// this value is chosen somewhat arbitrarily, and is not a limit for bytes read,
/// but is instead the amount of bytes to be read *at once*
const MAX_READ_BYTES: u32 = 2048;

/// Maximum bytes to write at a time, due to the limitations on wasi-io's blocking_write_and_flush()
const MAX_WRITE_BYTES: usize = 4096;

// The value is derived from th error message: "failed to compile component: memory index 0 has a minimum byte size of 2001076224 which exceeds the limit of 268435456 bytes"
const MAX_SIZE: usize = 240_000_000;
static mut DUMMY_BUFFER: [u8; MAX_SIZE] = [0; MAX_SIZE];

/// A one-time flag so we only fill the buffer once
static INIT: OnceCell<()> = OnceCell::new();

impl Guest for HttpServer {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        init_dummy_buffer();
        // Extract "/?size=NNN" from path_with_query(), or return 400 if absent
        let path_query = match request.path_with_query() {
            Some(p) => p,
            None => {
                let err_resp = error_response(400, "No query string found. Try /?size=NNN");
                ResponseOutparam::set(response_out, Ok(err_resp));
                return;
            }
        };

        // Split on '=' and expect something like ["/?size", "12345"]
        // If the pattern doesn't match, return 400
        let parts = path_query.split('=').collect::<Vec<&str>>();
        let requested_size = match parts[..] {
            ["/?size", size_str] => {
                // Attempt to parse the size as usize
                match size_str.trim().parse::<usize>() {
                    Ok(val) => val,
                    Err(e) => {
                        let msg = format!("Failed to parse integer after size=, got '{size_str}': {e}");
                        let err_resp = error_response(400, &msg);
                        ResponseOutparam::set(response_out, Ok(err_resp));
                        return;
                    }
                }
            }
            _ => {
                // If we don't get exactly ["/?size", something], raise error
                let err_resp = error_response(
                    400,
                    &format!("Query must be /?size=NNN, got '{path_query}'"),
                );
                ResponseOutparam::set(response_out, Ok(err_resp));
                return;
            }
        };

        if requested_size > MAX_SIZE {
            let msg = format!("Requested size {requested_size} > MAX_SIZE {MAX_SIZE}");
            let err_resp = error_response(400, &msg);
            ResponseOutparam::set(response_out, Ok(err_resp));
            return;
        }

        // Do something with the buffer to avoid buffer being optimized away, but do NOT pass it to ping

        let slice = unsafe { &DUMMY_BUFFER[..requested_size] };
        let sum = slice.iter().fold(0_u64, |acc, &b| acc + b as u64);

        // let payload = String::from_utf8_lossy(slice);
        
        // Minimally pass a tiny string to ping
        let payload = "";

        let start_time = monotonic_clock::now();
        let pong = example::pong::pingpong::ping(payload);
        let end_time = monotonic_clock::now();
        let elapsed_time_ns = end_time - start_time;

        let response = OutgoingResponse::new(Fields::new());
        response.set_status_code(200).unwrap();

        let response_text = format!(
            "Hello! I got pong={pong}\n\
             Requested size: {requested_size}\n\
             Sum of entire buffer: {sum}\n\
             Start timestamp: {start_time}\n\
             End timestamp: {end_time}\n\
             Elapsed time (ns): {elapsed_time_ns}\n"
        );
        response
            .send_body(response_text.as_bytes())
            .expect("failed to send response body");

        ResponseOutparam::set(response_out, Ok(response));
    }
}

/// Initialize the DUMMY_BUFFER once with non-zero data
/// This approach ensures the memory is actually allocated and each byte is touched
fn init_dummy_buffer() {
    // If we've already done this, do nothing
    if INIT.get().is_some() {
        return;
    }

    INIT.get_or_init(|| {
        unsafe {
            for i in 0..MAX_SIZE {
                // A simple filler: store i % 256
                // (a real RNG might require WASI random APIs)
                DUMMY_BUFFER[i] = (i & 0xFF) as u8;
            }
        }
    });
}

fn error_response(status_code: u16, msg: &str) -> OutgoingResponse {
    let err_resp = OutgoingResponse::new(Fields::new());
    err_resp.set_status_code(status_code).unwrap();
    err_resp
        .send_body(msg.as_bytes())
        .expect("failed to send error response");
    err_resp
}

// NOTE: Since wit-bindgen makes IncomingRequest available to us as a local type,
// we can add convenience functions to it
impl IncomingRequest {
    /// This is a convenience function that writes out the body of a IncomingRequest (from wasi:http)
    /// into anything that supports [std::io::Write]
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

// NOTE: Since wit-bindgen makes OutgoingBody available to us as a local type,
// we can add convenience functions to it
impl OutgoingResponse {
    /// This is a convenience function that writes out the body of a IncomingRequest (from wasi:http)
    /// into anything that supports [std::io::Read]
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


