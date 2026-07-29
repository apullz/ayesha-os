fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("src/ayesha.ico");
        let _ = res.compile();
    }
}
