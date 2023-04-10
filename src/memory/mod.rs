pub mod process;
pub mod signature;

mod error;

use cfg_if;

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        mod linux;
    } else if #[cfg(windows)] {
        mod windows;
        use self::windows::*;
    } 
}
