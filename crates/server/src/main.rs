fn main() {
    glossa_server::run_blocking(std::env::args().skip(1).collect());
}
