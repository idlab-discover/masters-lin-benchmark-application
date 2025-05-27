# Benchmarking App for wasmCloud Performance Evaluation

This repository provides a minimal benchmarking application designed to evaluate invocation models in wasmCloud.

## Repository Structure

- `main`: Simplified benchmark that uses a hardcoded payload size due to wasmCloud host/HTTP server provider limitations.
- `feat/complete_http_server`: Implements a fully functional HTTP server component.
- `feat/pass_effective_size_by_request_body`: Implement an HTTP server component that receives a payload size dynamically via the HTTP request body.

## Application Structure

- **http-component**: Receives HTTP requests and invokes the `ping` method on `pong-component`.
- **pong-component**: Echoes the received input string.
- **HTTP server provider**: Implements the `wasi:http/incoming-handler` interface to receive HTTP(S) requests and trigger the workflow.

## Build-time Composition

To build a statically composed WebAssembly component using [`wac`](https://github.com/bytecodealliance/wac), run the following from the repository root:

```
wac plug --plug ./pong/build/pong_s.wasm ./http-hello2/build/http_hello_world_s.wasm -o composed.wasm
```

Ensure you have `wac` installed and run this command from the root directory of the project.

## Known Issues

- [wasmCloud issue #4004](https://github.com/wasmCloud/wasmCloud/issues/4004): The wasmCloud host/HTTP server provider cannot parse sequential request bodies correctly. As a workaround, the payload size is hardcoded in the benchmarking component.
- [wasm-tools bug](https://github.com/bytecodealliance/wasm-tools/pull/1999)/[wasmtime-wasi failure](https://github.com/bytecodealliance/wasmtime/issues/10184): After updating wit-parser to solve the issue, some v0.2.0 interfaces may fall.

## Reference

This benchmarking application is adapted from the official [wasmCloud composition example](https://github.com/wasmCloud/wasmCloud/tree/main/examples/rust/composition).
