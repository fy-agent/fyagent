#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub(crate) use macos::MacOsSecretBackend as NativeSecretBackend;
#[cfg(target_os = "windows")]
pub(crate) use windows::WindowsSecretBackend as NativeSecretBackend;
