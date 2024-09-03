#[derive(PartialEq, Eq)]
pub enum ColorSupport {
    Supported,
    NotSupported,
}

/// Return whether or not the current environment supports ANSI color output.
pub fn is_color_supported() -> ColorSupport {
    use supports_color::Stream;
    if supports_color::on(Stream::Stdout).is_some() {
        ColorSupport::Supported
    } else {
        ColorSupport::NotSupported
    }
}
