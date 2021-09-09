fn main() {
    #[cfg(feature = "ffi")]
    {
        uniffi_build::generate_scaffolding("./src/client.udl").unwrap();
    }
}
