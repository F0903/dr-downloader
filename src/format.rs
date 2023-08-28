use std::borrow::Cow;

pub struct Format<'a> {
    format_ext: Cow<'a, str>,
}

impl<'a> Format<'a> {
    pub fn new(name_or_ext: impl AsRef<str>) -> Self {
        let mut format = name_or_ext.as_ref().to_owned();
        if !format.starts_with('.') {
            format.insert(0, '.');
        }

        Self {
            format_ext: Cow::Owned(format),
        }
    }

    /// Construct format with EXACT format extension on comptime.
    pub const fn from_exact_extension(ext: &'a str) -> Self {
        Self {
            format_ext: Cow::Borrowed(ext),
        }
    }

    pub fn get_extension(&self) -> &str {
        &self.format_ext
    }
}
