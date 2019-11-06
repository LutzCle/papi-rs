fn main() {
    let path = std::path::Path::new("examples/configuration.toml");
    let config = papi::Config::parse_file(path).unwrap();
    assert!(papi::Papi::init_with_config(config).is_ok());
}
