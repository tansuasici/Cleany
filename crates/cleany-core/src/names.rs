//! Lossless name storage for the arena tree.
//!
//! Names are packed into one contiguous buffer and addressed by
//! `(offset, len)`. The unit is native to the OS so round-tripping needs no
//! `unsafe` and never loses invalid UTF-8 (Unix) or unpaired surrogates
//! (Windows): bytes on Unix, UTF-16 code units on Windows.

use std::ffi::{OsStr, OsString};

use crate::tree::TreeError;

#[cfg(not(any(unix, windows)))]
compile_error!("cleany-core supports Unix and Windows targets only");

#[cfg(unix)]
type Unit = u8;
#[cfg(windows)]
type Unit = u16;

/// Packed, append-only storage for entry names.
#[derive(Debug, Default)]
pub(crate) struct NamePool {
    buf: Vec<Unit>,
}

impl NamePool {
    /// Appends `name` and returns its `(offset, len)` in pool units.
    pub(crate) fn push(&mut self, name: &OsStr) -> Result<(u32, u16), TreeError> {
        let start = self.buf.len();
        extend_units(&mut self.buf, name);
        let len = self.buf.len() - start;
        let Ok(len16) = u16::try_from(len) else {
            self.buf.truncate(start);
            return Err(TreeError::NameTooLong);
        };
        let Ok(off) = u32::try_from(start) else {
            self.buf.truncate(start);
            return Err(TreeError::NamePoolFull);
        };
        if u32::try_from(self.buf.len()).is_err() {
            self.buf.truncate(start);
            return Err(TreeError::NamePoolFull);
        }
        Ok((off, len16))
    }

    /// Returns the name stored at `(off, len)`.
    pub(crate) fn get(&self, off: u32, len: u16) -> OsString {
        let start = off as usize;
        let end = start + usize::from(len);
        self.buf
            .get(start..end)
            .map_or_else(OsString::new, from_units)
    }
}

#[cfg(unix)]
fn extend_units(buf: &mut Vec<Unit>, name: &OsStr) {
    use std::os::unix::ffi::OsStrExt;
    buf.extend_from_slice(name.as_bytes());
}

#[cfg(windows)]
fn extend_units(buf: &mut Vec<Unit>, name: &OsStr) {
    use std::os::windows::ffi::OsStrExt;
    buf.extend(name.encode_wide());
}

#[cfg(unix)]
fn from_units(units: &[Unit]) -> OsString {
    use std::os::unix::ffi::OsStrExt;
    OsStr::from_bytes(units).to_os_string()
}

#[cfg(windows)]
fn from_units(units: &[Unit]) -> OsString {
    use std::os::windows::ffi::OsStringExt;
    OsString::from_wide(units)
}

#[cfg(test)]
mod tests {
    use super::NamePool;
    use std::ffi::{OsStr, OsString};

    #[cfg(unix)]
    fn non_unicode_name() -> OsString {
        use std::os::unix::ffi::OsStrExt;
        OsStr::from_bytes(b"caf\xe9-\xff.txt").to_os_string()
    }

    #[cfg(windows)]
    fn non_unicode_name() -> OsString {
        use std::os::windows::ffi::OsStringExt;
        // Unpaired high surrogate followed by "a".
        OsString::from_wide(&[0xD800, u16::from(b'a')])
    }

    #[test]
    fn name_pool_roundtrips_non_utf8() {
        let name = non_unicode_name();
        assert!(name.to_str().is_none(), "fixture must not be valid Unicode");
        let mut pool = NamePool::default();
        let (off, len) = pool.push(&name).unwrap();
        assert_eq!(pool.get(off, len), name);
    }

    #[test]
    fn name_pool_keeps_names_apart() {
        let mut pool = NamePool::default();
        let a = pool.push(OsStr::new("alpha")).unwrap();
        let b = pool.push(OsStr::new("ünïcødé 名前")).unwrap();
        assert_eq!(pool.get(a.0, a.1), OsStr::new("alpha"));
        assert_eq!(pool.get(b.0, b.1), OsStr::new("ünïcødé 名前"));
    }

    #[test]
    fn name_pool_rejects_too_long_name() {
        let mut pool = NamePool::default();
        let long = "x".repeat(usize::from(u16::MAX) + 1);
        assert!(pool.push(OsStr::new(&long)).is_err());
        assert_eq!(pool.push(OsStr::new("ok")).unwrap(), (0, 2));
    }
}
