fn main() {
    #[cfg(feature = "ffi")]
    {
        uniffi::generate_scaffolding("./src/client.udl").unwrap();
    }
}
