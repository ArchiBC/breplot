//! Typst minimal protocol bridge, implemented directly to keep the crate dependency-free.
#[link(wasm_import_module = "typst_env")]
unsafe extern "C" {
    fn wasm_minimal_protocol_send_result_to_host(ptr: *const u8, len: usize);
    fn wasm_minimal_protocol_write_args_to_buffer(ptr: *mut u8);
}
// Host supplies the byte length and copies arguments into the allocated buffer.
#[unsafe(no_mangle)]
pub extern "C" fn surface_demo(length: usize) -> i32 {
    if length > 128 {
        return send(Err("scene id is too long".into()));
    }
    let mut input = vec![0; length];
    unsafe {
        wasm_minimal_protocol_write_args_to_buffer(input.as_mut_ptr());
    }
    send(
        std::str::from_utf8(&input)
            .map_err(|e| e.to_string())
            .and_then(|id| crate::demo::response(id).map_err(|e| e.to_string())),
    )
}
fn send(result: Result<Vec<u8>, String>) -> i32 {
    let (bytes, status) = match result {
        Ok(b) => (b, 0),
        Err(e) => (e.into_bytes(), 1),
    };
    unsafe {
        wasm_minimal_protocol_send_result_to_host(bytes.as_ptr(), bytes.len());
    }
    status
}
