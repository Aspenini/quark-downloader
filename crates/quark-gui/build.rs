fn main() {
    #[cfg(feature = "slint")]
    {
        slint_build::compile("ui/app.slint").expect("compile Slint UI");
    }
}
