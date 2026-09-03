#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod unavailable;

#[cfg(target_os = "macos")]
pub(crate) use macos::MacOsSecretBackend as NativeSecretBackend;
#[cfg(target_os = "windows")]
pub(crate) use windows::WindowsSecretBackend as NativeSecretBackend;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) use unavailable::UnavailableSecretBackend as NativeSecretBackend;
