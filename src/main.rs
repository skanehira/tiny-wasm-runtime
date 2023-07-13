use miniwasm::Module;

fn main() -> anyhow::Result<()> {
    let bytes = include_bytes!("./fixtures/hello_world.wasm");
    let module = Module::new(bytes)?;
    println!("{:?}", module);
    Ok(())
}
