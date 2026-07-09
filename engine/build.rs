fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("src/ayesha.ico");
        res.compile().expect("Failed to compile Windows resource");
    }
}
