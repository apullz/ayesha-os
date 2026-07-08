fn main() {
    #[cfg(windows)]
    {
        let _ = std::panic::catch_unwind(|| {
            embed_resource::compile("src/ayesha.rc", None::<String>);
        });
    }
}
