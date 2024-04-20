use anyhow::Result;
use tinywasm::execution::{runtime::Runtime, wasi::WasiSnapshotPreview1};

fn main() -> Result<()> {
    let wasi = WasiSnapshotPreview1::new();
    let wasm = include_bytes!("./fixtures/hello_world.wasm");
    let mut runtime = Runtime::instantiate_with_wasi(wasm, wasi)?;
    runtime.call("_start", vec![]).unwrap();
    Ok(())
}
