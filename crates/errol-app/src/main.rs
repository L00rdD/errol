fn main() {
    if let Err(error) = errol_app::run() {
        eprintln!("errol: {error}");
        std::process::exit(1);
    }
}
