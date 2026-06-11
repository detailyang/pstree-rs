/// Returns the terminal width in columns, or None if stdout is not a tty
/// or the width cannot be determined.
pub fn terminal_width() -> Option<usize> {
    #[cfg(unix)]
    {
        use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ};
        let mut ws = winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let ret = unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut ws) };
        if ret == 0 && ws.ws_col > 0 {
            return Some(ws.ws_col as usize);
        }
    }
    None
}
