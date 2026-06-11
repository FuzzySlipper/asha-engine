// napi-rs build setup. Only runs when this crate is built explicitly with a
// native toolchain (it is excluded from the offline workspace build).
fn main() {
    napi_build::setup();
}
