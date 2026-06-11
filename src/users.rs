/// Resolve a username to a uid via getpwnam(3).
/// Returns None if the user is not found.
pub fn username_to_uid(name: &str) -> Option<u32> {
    use std::ffi::CString;
    let cname = CString::new(name).ok()?;
    let pw = unsafe { libc::getpwnam(cname.as_ptr()) };
    if pw.is_null() {
        return None;
    }
    Some(unsafe { (*pw).pw_uid })
}
