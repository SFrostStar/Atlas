//! Safe access to the user's number-format separator for Java-compatible UI.
use std::sync::OnceLock;

pub fn decimal_separator() -> &'static str {
    static SEPARATOR: OnceLock<String> = OnceLock::new();
    SEPARATOR.get_or_init(read_decimal_separator)
}

#[cfg(windows)]
fn read_decimal_separator() -> String {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLocaleInfoEx(locale: *const u16, kind: u32, data: *mut u16, length: i32) -> i32;
    }
    let mut buffer = [0u16; 8];
    // SAFETY: null selects the user locale; the writable buffer has the supplied capacity.
    let count = unsafe {
        GetLocaleInfoEx(
            std::ptr::null(),
            14,
            buffer.as_mut_ptr(),
            buffer.len() as i32,
        )
    };
    separator_from_utf16(&buffer, count).unwrap_or_else(|| ".".into())
}

#[cfg(not(windows))]
fn read_decimal_separator() -> String {
    ".".into()
}

#[cfg(any(windows, test))]
fn separator_from_utf16(buffer: &[u16], count: i32) -> Option<String> {
    let count = usize::try_from(count).ok()?;
    if count <= 1 || count > buffer.len() || buffer[count - 1] != 0 {
        return None;
    }
    String::from_utf16(&buffer[..count - 1]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_system_separator_and_failure() {
        assert_eq!(separator_from_utf16(&[44, 0], 2).as_deref(), Some(","));
        assert_eq!(separator_from_utf16(&[46, 0], 2).as_deref(), Some("."));
        for count in [-1, 0, 1, 3] {
            assert!(separator_from_utf16(&[44, 0], count).is_none());
        }
        assert!(separator_from_utf16(&[0xd800, 0], 2).is_none());
        assert!(!decimal_separator().is_empty());
    }
}
