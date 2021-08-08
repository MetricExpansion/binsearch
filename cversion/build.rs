fn main() {
    cc::Build::new()
        .cpp(true)
        .file("src/cversion.cpp")
        .compile("cversion");
}
