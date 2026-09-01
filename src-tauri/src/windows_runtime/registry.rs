//! Symbolic-link-safe access to the frozen Shell user's registry hive.
//!
//! The only supported locations are enumerated below. Registry paths never
//! cross the renderer boundary, and each component is opened relative to an
//! already verified handle with `REG_OPTION_OPEN_LINK` so a registry link
//! cannot redirect an elevated operation into another hive.

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellUserRegistryLocation {
    Environment,
    Run,
    Uninstall,
    AppPaths,
}

#[cfg(any(target_os = "windows", test))]
impl ShellUserRegistryLocation {
    fn components(self, canonical_sid: &str) -> Vec<&str> {
        match self {
            Self::Environment => vec![canonical_sid, "Environment"],
            Self::Run => vec![
                canonical_sid,
                "Software",
                "Microsoft",
                "Windows",
                "CurrentVersion",
                "Run",
            ],
            Self::Uninstall => vec![
                canonical_sid,
                "Software",
                "Microsoft",
                "Windows",
                "CurrentVersion",
                "Uninstall",
            ],
            Self::AppPaths => vec![
                canonical_sid,
                "Software",
                "Microsoft",
                "Windows",
                "CurrentVersion",
                "App Paths",
            ],
        }
    }
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MachineRegistryLocation {
    Uninstall,
    AppPaths,
}

#[cfg(any(target_os = "windows", test))]
impl MachineRegistryLocation {
    fn components(self) -> &'static [&'static str] {
        match self {
            Self::Uninstall => &[
                "Software",
                "Microsoft",
                "Windows",
                "CurrentVersion",
                "Uninstall",
            ],
            Self::AppPaths => &[
                "Software",
                "Microsoft",
                "Windows",
                "CurrentVersion",
                "App Paths",
            ],
        }
    }
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RegistryView {
    Native,
    Registry32,
    Registry64,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RegistryRights {
    query_value: bool,
    enumerate_subkeys: bool,
    create_subkey: bool,
    set_value: bool,
}

#[cfg(any(target_os = "windows", test))]
impl RegistryRights {
    const READ_VALUES: Self = Self {
        query_value: true,
        enumerate_subkeys: false,
        create_subkey: false,
        set_value: false,
    };

    const UPDATE_VALUES: Self = Self {
        query_value: true,
        enumerate_subkeys: false,
        create_subkey: false,
        set_value: true,
    };

    const TRAVERSE: Self = Self {
        query_value: true,
        enumerate_subkeys: true,
        create_subkey: false,
        set_value: false,
    };

    /// Read-only access for an inventory parent whose direct children must be
    /// enumerated. Keep this semantic capability separate from traversal so a
    /// caller cannot accidentally downgrade the leaf to query-only access.
    const INVENTORY_PARENT_READ: Self = Self {
        query_value: true,
        enumerate_subkeys: true,
        create_subkey: false,
        set_value: false,
    };

    const fn with_create_subkey(mut self) -> Self {
        self.create_subkey = true;
        self
    }
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreateDisposition {
    CreatedNew,
    OpenedExisting,
}

#[cfg(any(target_os = "windows", test))]
trait RegistryBackend {
    type Handle;
    type Error;

    fn root(&self) -> Self::Handle;

    fn open_link(
        &mut self,
        parent: &Self::Handle,
        component: &str,
        rights: RegistryRights,
    ) -> Result<Self::Handle, Self::Error>;

    fn create_or_open(
        &mut self,
        parent: &Self::Handle,
        component: &str,
        rights: RegistryRights,
    ) -> Result<(Self::Handle, CreateDisposition), Self::Error>;

    fn has_symbolic_link_value(&mut self, handle: &Self::Handle) -> Result<bool, Self::Error>;
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, PartialEq, Eq)]
enum RegistryTraversalError<E> {
    Backend(E),
    SymbolicLinkComponent,
}

#[cfg(any(target_os = "windows", test))]
fn verify_not_symbolic_link<B: RegistryBackend>(
    backend: &mut B,
    handle: &B::Handle,
) -> Result<(), RegistryTraversalError<B::Error>> {
    if backend
        .has_symbolic_link_value(handle)
        .map_err(RegistryTraversalError::Backend)?
    {
        return Err(RegistryTraversalError::SymbolicLinkComponent);
    }
    Ok(())
}

/// Traverse one fixed registry location component-by-component. When creating
/// the leaf, an existing handle returned by `RegCreateKeyExW` is deliberately
/// discarded because that API can follow a registry link. The leaf is then
/// reopened relative to the pinned parent with `REG_OPTION_OPEN_LINK`.
#[cfg(any(target_os = "windows", test))]
fn traverse_components<B: RegistryBackend>(
    backend: &mut B,
    components: &[&str],
    final_rights: RegistryRights,
    create_leaf: bool,
) -> Result<B::Handle, RegistryTraversalError<B::Error>> {
    debug_assert!(!components.is_empty());

    let mut current = backend.root();
    for (index, component) in components.iter().enumerate() {
        let is_leaf = index + 1 == components.len();
        let rights = if is_leaf {
            final_rights
        } else if create_leaf && index + 2 == components.len() {
            RegistryRights::TRAVERSE.with_create_subkey()
        } else {
            RegistryRights::TRAVERSE
        };

        let next = if is_leaf && create_leaf {
            let (created_handle, disposition) = backend
                .create_or_open(&current, component, rights)
                .map_err(RegistryTraversalError::Backend)?;
            match disposition {
                CreateDisposition::CreatedNew => created_handle,
                CreateDisposition::OpenedExisting => {
                    drop(created_handle);
                    backend
                        .open_link(&current, component, rights)
                        .map_err(RegistryTraversalError::Backend)?
                }
            }
        } else {
            backend
                .open_link(&current, component, rights)
                .map_err(RegistryTraversalError::Backend)?
        };

        verify_not_symbolic_link(backend, &next)?;
        current = next;
    }

    Ok(current)
}

#[cfg(any(target_os = "windows", test))]
fn open_known_location<B: RegistryBackend>(
    backend: &mut B,
    canonical_sid: &str,
    location: ShellUserRegistryLocation,
    final_rights: RegistryRights,
    create_leaf: bool,
) -> Result<B::Handle, RegistryTraversalError<B::Error>> {
    traverse_components(
        backend,
        &location.components(canonical_sid),
        final_rights,
        create_leaf,
    )
}

#[cfg(target_os = "windows")]
mod windows {
    use std::io;

    use winreg::{
        enums::{
            HKEY_LOCAL_MACHINE, HKEY_USERS, KEY_CREATE_SUB_KEY, KEY_ENUMERATE_SUB_KEYS,
            KEY_QUERY_VALUE, KEY_SET_VALUE, KEY_WOW64_32KEY, KEY_WOW64_64KEY, REG_CREATED_NEW_KEY,
            REG_LINK, REG_OPENED_EXISTING_KEY, REG_OPTION_NON_VOLATILE, REG_OPTION_OPEN_LINK,
        },
        RegKey, HKEY,
    };

    use super::{
        open_known_location, traverse_components, CreateDisposition, MachineRegistryLocation,
        RegistryBackend, RegistryRights, RegistryTraversalError, RegistryView,
        ShellUserRegistryLocation,
    };

    struct WindowsRegistryBackend {
        root: HKEY,
        view: RegistryView,
    }

    impl WindowsRegistryBackend {
        const fn new(root: HKEY) -> Self {
            Self {
                root,
                view: RegistryView::Native,
            }
        }

        const fn with_view(root: HKEY, view: RegistryView) -> Self {
            Self { root, view }
        }
    }

    impl RegistryRights {
        fn windows_mask(self, view: RegistryView) -> u32 {
            let mut mask = 0;
            if self.query_value {
                mask |= KEY_QUERY_VALUE;
            }
            if self.enumerate_subkeys {
                mask |= KEY_ENUMERATE_SUB_KEYS;
            }
            if self.create_subkey {
                mask |= KEY_CREATE_SUB_KEY;
            }
            if self.set_value {
                mask |= KEY_SET_VALUE;
            }
            mask | match view {
                RegistryView::Native => 0,
                RegistryView::Registry32 => KEY_WOW64_32KEY,
                RegistryView::Registry64 => KEY_WOW64_64KEY,
            }
        }
    }

    impl RegistryBackend for WindowsRegistryBackend {
        type Handle = RegKey;
        type Error = io::Error;

        fn root(&self) -> Self::Handle {
            RegKey::predef(self.root)
        }

        fn open_link(
            &mut self,
            parent: &Self::Handle,
            component: &str,
            rights: RegistryRights,
        ) -> Result<Self::Handle, Self::Error> {
            parent.open_subkey_with_options_flags(
                component,
                REG_OPTION_OPEN_LINK,
                rights.windows_mask(self.view),
            )
        }

        fn create_or_open(
            &mut self,
            parent: &Self::Handle,
            component: &str,
            rights: RegistryRights,
        ) -> Result<(Self::Handle, CreateDisposition), Self::Error> {
            let (handle, disposition) = parent.create_subkey_with_options_flags(
                component,
                REG_OPTION_NON_VOLATILE,
                rights.windows_mask(self.view),
            )?;
            let disposition = match disposition {
                REG_CREATED_NEW_KEY => CreateDisposition::CreatedNew,
                REG_OPENED_EXISTING_KEY => CreateDisposition::OpenedExisting,
            };
            Ok((handle, disposition))
        }

        fn has_symbolic_link_value(&mut self, handle: &Self::Handle) -> Result<bool, Self::Error> {
            match handle.get_raw_value("SymbolicLinkValue") {
                Ok(value) if value.vtype == REG_LINK => Ok(true),
                // Treat a marker with an unexpected type as hostile too. This
                // avoids relying on an attacker's value type remaining stable.
                Ok(_) => Ok(true),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(error),
            }
        }
    }

    fn map_security_error(error: RegistryTraversalError<io::Error>) -> io::Error {
        match error {
            RegistryTraversalError::Backend(error) => error,
            RegistryTraversalError::SymbolicLinkComponent => io::Error::new(
                io::ErrorKind::PermissionDenied,
                "registry symbolic-link component rejected",
            ),
        }
    }

    fn open_shell_location(
        location: ShellUserRegistryLocation,
        rights: RegistryRights,
        create_leaf: bool,
    ) -> io::Result<RegKey> {
        let canonical_sid = super::super::require_interactive_user_context().canonical_sid();
        let mut backend = WindowsRegistryBackend::new(HKEY_USERS);
        open_known_location(&mut backend, canonical_sid, location, rights, create_leaf)
            .map_err(map_security_error)
    }

    pub(crate) fn open_shell_user_environment_read() -> io::Result<RegKey> {
        open_shell_location(
            ShellUserRegistryLocation::Environment,
            RegistryRights::READ_VALUES,
            false,
        )
    }

    pub(crate) fn open_shell_user_environment_update() -> io::Result<RegKey> {
        open_shell_location(
            ShellUserRegistryLocation::Environment,
            RegistryRights::UPDATE_VALUES,
            false,
        )
    }

    pub(crate) fn create_or_open_shell_user_environment_update() -> io::Result<RegKey> {
        open_shell_location(
            ShellUserRegistryLocation::Environment,
            RegistryRights::UPDATE_VALUES,
            true,
        )
    }

    pub(crate) fn open_shell_user_run_update() -> io::Result<RegKey> {
        open_shell_location(
            ShellUserRegistryLocation::Run,
            RegistryRights::UPDATE_VALUES,
            false,
        )
    }

    pub(crate) fn open_shell_user_inventory_parent(
        location: ShellUserRegistryLocation,
        view: RegistryView,
    ) -> io::Result<RegKey> {
        if !matches!(
            location,
            ShellUserRegistryLocation::Uninstall | ShellUserRegistryLocation::AppPaths
        ) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "registry location is not an inventory parent",
            ));
        }
        let canonical_sid = super::super::require_interactive_user_context().canonical_sid();
        let mut backend = WindowsRegistryBackend::with_view(HKEY_USERS, view);
        open_known_location(
            &mut backend,
            canonical_sid,
            location,
            RegistryRights::INVENTORY_PARENT_READ,
            false,
        )
        .map_err(map_security_error)
    }

    pub(crate) fn open_machine_inventory_parent(
        location: MachineRegistryLocation,
        view: RegistryView,
    ) -> io::Result<RegKey> {
        let mut backend = WindowsRegistryBackend::with_view(HKEY_LOCAL_MACHINE, view);
        traverse_components(
            &mut backend,
            location.components(),
            RegistryRights::INVENTORY_PARENT_READ,
            false,
        )
        .map_err(map_security_error)
    }

    pub(crate) fn open_inventory_child_read(
        parent: &RegKey,
        component: &str,
        view: RegistryView,
    ) -> io::Result<RegKey> {
        if component.is_empty()
            || component.len() > 512
            || component.contains(['\\', '/', '\0'])
            || component.chars().any(char::is_control)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "registry child is not one bounded component",
            ));
        }
        let mut backend = WindowsRegistryBackend::with_view(parent.raw_handle(), view);
        let child = backend.open_link(parent, component, RegistryRights::READ_VALUES)?;
        match backend.has_symbolic_link_value(&child) {
            Ok(false) => Ok(child),
            Ok(true) => Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "registry symbolic-link component rejected",
            )),
            Err(error) => Err(error),
        }
    }

    #[cfg(test)]
    mod native_tests {
        use std::io;
        use std::os::windows::ffi::OsStrExt;

        use uuid::Uuid;
        use windows::{
            core::{PCWSTR, PWSTR},
            Wdk::System::Registry::NtDeleteKey,
            Win32::{
                Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL},
                Security::{
                    Authorization::ConvertSidToStringSidW, GetTokenInformation, TokenUser,
                    TOKEN_QUERY, TOKEN_USER,
                },
                System::Threading::{GetCurrentProcess, OpenProcessToken},
            },
        };
        use winreg::{
            enums::{
                HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_LINK,
                REG_OPTION_CREATE_LINK, REG_OPTION_OPEN_LINK,
            },
            RegKey, RegValue,
        };

        use super::super::traverse_components;
        use super::*;

        struct OwnedTestHandle(HANDLE);

        impl Drop for OwnedTestHandle {
            fn drop(&mut self) {
                if !self.0.is_invalid() {
                    unsafe {
                        let _ = CloseHandle(self.0);
                    }
                }
            }
        }

        fn current_process_sid() -> String {
            let mut token = HANDLE::default();
            unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
                .expect("current process token must be queryable");
            let token = OwnedTestHandle(token);

            let mut required = 0_u32;
            let _ = unsafe { GetTokenInformation(token.0, TokenUser, None, 0, &mut required) };
            assert!(required > 0, "TokenUser must report a buffer size");

            let word = std::mem::size_of::<usize>();
            let mut aligned = vec![0_usize; (required as usize).div_ceil(word)];
            unsafe {
                GetTokenInformation(
                    token.0,
                    TokenUser,
                    Some(aligned.as_mut_ptr().cast()),
                    required,
                    &mut required,
                )
            }
            .expect("TokenUser must be readable");
            let token_user = unsafe { &*aligned.as_ptr().cast::<TOKEN_USER>() };

            let mut string_sid = PWSTR::null();
            unsafe { ConvertSidToStringSidW(token_user.User.Sid, &mut string_sid) }
                .expect("current process SID must be renderable");
            assert!(!string_sid.is_null(), "rendered SID must not be null");
            let rendered = unsafe { PCWSTR(string_sid.0).to_string() }
                .expect("rendered SID must be valid UTF-16");
            unsafe {
                let _ = LocalFree(Some(HLOCAL(string_sid.0.cast())));
            }
            rendered
        }

        struct IsolatedRegistryTree {
            path: String,
            parent: Option<RegKey>,
        }

        impl IsolatedRegistryTree {
            fn parent(&self) -> &RegKey {
                self.parent
                    .as_ref()
                    .expect("isolated registry tree must still be open")
            }

            fn cleanup(&mut self) -> io::Result<()> {
                let mut first_error = None;
                if let Some(parent) = self.parent.take() {
                    for link_name in ["LeafLink", "IntermediateLink"] {
                        match delete_registry_link(&parent, link_name) {
                            Ok(()) => {}
                            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                            Err(error) if first_error.is_none() => first_error = Some(error),
                            Err(_) => {}
                        }
                    }
                    drop(parent);
                }
                match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) if first_error.is_none() => first_error = Some(error),
                    Err(_) => {}
                }
                first_error.map_or(Ok(()), Err)
            }
        }

        fn delete_registry_link(parent: &RegKey, name: &str) -> io::Result<()> {
            const DELETE_ACCESS: u32 = 0x0001_0000;
            let link =
                parent.open_subkey_with_options_flags(name, REG_OPTION_OPEN_LINK, DELETE_ACCESS)?;
            unsafe { NtDeleteKey(HANDLE(link.raw_handle())) }
                .ok()
                .map_err(io::Error::other)
        }

        impl Drop for IsolatedRegistryTree {
            fn drop(&mut self) {
                let _ = self.cleanup();
            }
        }

        fn create_registry_link(parent: &RegKey, name: &str, target_object_name: &str) {
            let (link, _) = parent
                .create_subkey_with_options_flags(
                    name,
                    REG_OPTION_CREATE_LINK,
                    KEY_QUERY_VALUE | KEY_SET_VALUE,
                )
                .expect("isolated registry link must be creatable");
            let target: Vec<u16> = std::ffi::OsStr::new(target_object_name)
                .encode_wide()
                .collect();
            let bytes: Vec<u8> = target.iter().flat_map(|word| word.to_le_bytes()).collect();
            assert!(
                !bytes.ends_with(&[0, 0]),
                "REG_LINK Object Name must not be NUL terminated"
            );
            link.set_raw_value(
                "SymbolicLinkValue",
                &RegValue {
                    bytes,
                    vtype: REG_LINK,
                },
            )
            .expect("registry link target must be writable");
        }

        #[test]
        fn isolated_native_open_link_accepts_normal_keys_and_rejects_link_components() {
            let test_id = Uuid::new_v4().simple().to_string();
            let test_root = format!(r"Software\FyAgentRegistrySecurityTests\{test_id}");
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let (parent, _) = hkcu
                .create_subkey(&test_root)
                .expect("isolated test registry parent must be creatable");
            let mut tree = IsolatedRegistryTree {
                path: test_root,
                parent: Some(parent),
            };
            let (target, _) = tree
                .parent()
                .create_subkey("Target")
                .expect("disposable link target must be creatable");
            target
                .set_value("Marker", &"target-root-unchanged")
                .expect("disposable target marker must be writable");
            let (target_nested, _) = target
                .create_subkey("Nested")
                .expect("disposable nested target must be creatable");
            target_nested
                .set_value("Marker", &"target-nested-unchanged")
                .expect("disposable nested marker must be writable");
            drop(target_nested);
            drop(target);
            tree.parent()
                .create_subkey(r"Normal\Leaf")
                .expect("ordinary isolated registry keys must be creatable");
            let target_object_name = format!(
                r"\Registry\User\{}\{}\Target",
                current_process_sid(),
                tree.path
            );
            create_registry_link(tree.parent(), "LeafLink", &target_object_name);
            create_registry_link(tree.parent(), "IntermediateLink", &target_object_name);

            let followed_leaf = tree
                .parent()
                .open_subkey("LeafLink")
                .expect("ordinary open must follow the disposable leaf link");
            assert_eq!(
                followed_leaf
                    .get_value::<String, _>("Marker")
                    .expect("followed leaf target marker must be readable"),
                "target-root-unchanged"
            );
            let followed_intermediate = tree
                .parent()
                .open_subkey(r"IntermediateLink\Nested")
                .expect("ordinary open must follow the disposable intermediate link");
            assert_eq!(
                followed_intermediate
                    .get_value::<String, _>("Marker")
                    .expect("followed intermediate target marker must be readable"),
                "target-nested-unchanged"
            );
            drop(followed_intermediate);
            drop(followed_leaf);

            let normal_components = [
                "Software",
                "FyAgentRegistrySecurityTests",
                test_id.as_str(),
                "Normal",
                "Leaf",
            ];
            let mut backend = WindowsRegistryBackend::new(HKEY_CURRENT_USER);
            let normal = traverse_components(
                &mut backend,
                &normal_components,
                RegistryRights::READ_VALUES,
                false,
            );
            assert!(normal.is_ok(), "REG_OPTION_OPEN_LINK must open normal keys");
            drop(normal);

            let leaf_components = [
                "Software",
                "FyAgentRegistrySecurityTests",
                test_id.as_str(),
                "LeafLink",
            ];
            let leaf_result = traverse_components(
                &mut backend,
                &leaf_components,
                RegistryRights::READ_VALUES,
                false,
            );
            assert!(matches!(
                leaf_result,
                Err(RegistryTraversalError::SymbolicLinkComponent)
            ));
            drop(leaf_result);

            let intermediate_components = [
                "Software",
                "FyAgentRegistrySecurityTests",
                test_id.as_str(),
                "IntermediateLink",
                "Nested",
            ];
            let intermediate_result = traverse_components(
                &mut backend,
                &intermediate_components,
                RegistryRights::READ_VALUES,
                false,
            );
            assert!(matches!(
                intermediate_result,
                Err(RegistryTraversalError::SymbolicLinkComponent)
            ));
            drop(intermediate_result);

            let target = tree
                .parent()
                .open_subkey("Target")
                .expect("disposable target must remain directly accessible");
            assert_eq!(
                target
                    .get_value::<String, _>("Marker")
                    .expect("root marker must remain readable"),
                "target-root-unchanged"
            );
            assert_eq!(
                target
                    .open_subkey("Nested")
                    .and_then(|nested| nested.get_value::<String, _>("Marker"))
                    .expect("nested marker must remain readable"),
                "target-nested-unchanged"
            );
            drop(target);

            tree.cleanup()
                .expect("isolated registry test tree must be deleted");
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) use windows::{
    create_or_open_shell_user_environment_update, open_inventory_child_read,
    open_machine_inventory_parent, open_shell_user_environment_read,
    open_shell_user_environment_update, open_shell_user_inventory_parent,
    open_shell_user_run_update,
};

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Event {
        Open {
            path: String,
            rights: RegistryRights,
        },
        Create {
            path: String,
            rights: RegistryRights,
        },
        Inspect(String),
    }

    struct FakeBackend {
        events: Vec<Event>,
        symbolic_paths: HashSet<String>,
        create_disposition: CreateDisposition,
        open_errors: HashMap<String, &'static str>,
        create_errors: HashMap<String, &'static str>,
        inspect_errors: HashMap<String, &'static str>,
    }

    impl Default for FakeBackend {
        fn default() -> Self {
            Self {
                events: Vec::new(),
                symbolic_paths: HashSet::new(),
                create_disposition: CreateDisposition::CreatedNew,
                open_errors: HashMap::new(),
                create_errors: HashMap::new(),
                inspect_errors: HashMap::new(),
            }
        }
    }

    impl RegistryBackend for FakeBackend {
        type Handle = String;
        type Error = &'static str;

        fn root(&self) -> Self::Handle {
            "HKEY_USERS".to_owned()
        }

        fn open_link(
            &mut self,
            parent: &Self::Handle,
            component: &str,
            rights: RegistryRights,
        ) -> Result<Self::Handle, Self::Error> {
            let path = format!(r"{parent}\{component}");
            self.events.push(Event::Open {
                path: path.clone(),
                rights,
            });
            if let Some(error) = self.open_errors.get(&path) {
                return Err(*error);
            }
            Ok(path)
        }

        fn create_or_open(
            &mut self,
            parent: &Self::Handle,
            component: &str,
            rights: RegistryRights,
        ) -> Result<(Self::Handle, CreateDisposition), Self::Error> {
            let path = format!(r"{parent}\{component}");
            self.events.push(Event::Create {
                path: path.clone(),
                rights,
            });
            if let Some(error) = self.create_errors.get(&path) {
                return Err(*error);
            }
            Ok((path, self.create_disposition))
        }

        fn has_symbolic_link_value(&mut self, handle: &Self::Handle) -> Result<bool, Self::Error> {
            self.events.push(Event::Inspect(handle.clone()));
            if let Some(error) = self.inspect_errors.get(handle) {
                return Err(*error);
            }
            Ok(self.symbolic_paths.contains(handle))
        }
    }

    const SID: &str = "S-1-5-21-100-200-300-1001";

    #[test]
    fn open_only_traverses_every_fixed_run_component_with_open_link() {
        let mut backend = FakeBackend::default();
        let handle = open_known_location(
            &mut backend,
            SID,
            ShellUserRegistryLocation::Run,
            RegistryRights::UPDATE_VALUES,
            false,
        )
        .expect("fixed Run path should open");

        assert_eq!(
            handle,
            format!(r"HKEY_USERS\{SID}\Software\Microsoft\Windows\CurrentVersion\Run")
        );
        let opened: Vec<_> = backend
            .events
            .iter()
            .filter_map(|event| match event {
                Event::Open { path, .. } => Some(path.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(opened.len(), 6);
        assert!(opened.iter().all(|path| handle.starts_with(path)));
        assert_eq!(
            backend
                .events
                .iter()
                .filter(|event| matches!(event, Event::Inspect(_)))
                .count(),
            6
        );
    }

    #[test]
    fn inventory_locations_are_fixed_component_chains_not_caller_paths() {
        for (location, expected) in [
            (
                ShellUserRegistryLocation::Uninstall,
                format!(r"HKEY_USERS\{SID}\Software\Microsoft\Windows\CurrentVersion\Uninstall"),
            ),
            (
                ShellUserRegistryLocation::AppPaths,
                format!(r"HKEY_USERS\{SID}\Software\Microsoft\Windows\CurrentVersion\App Paths"),
            ),
        ] {
            let mut backend = FakeBackend::default();
            let handle = open_known_location(
                &mut backend,
                SID,
                location,
                RegistryRights::INVENTORY_PARENT_READ,
                false,
            )
            .expect("fixed inventory parent should open");
            assert_eq!(handle, expected);
            let opens: Vec<_> = backend
                .events
                .iter()
                .filter_map(|event| match event {
                    Event::Open { path, rights } => Some((path, rights)),
                    Event::Inspect(_) => None,
                    Event::Create { .. } => panic!("inventory traversal must stay read-only"),
                })
                .collect();
            let (leaf_path, leaf_rights) = opens.last().expect("inventory leaf must open");
            assert_eq!(leaf_path.as_str(), expected);
            assert_eq!(**leaf_rights, RegistryRights::INVENTORY_PARENT_READ);
            assert!(leaf_rights.query_value);
            assert!(leaf_rights.enumerate_subkeys);
            assert!(!leaf_rights.create_subkey);
            assert!(!leaf_rights.set_value);
            assert!(opens[..opens.len() - 1]
                .iter()
                .all(|(_, rights)| **rights == RegistryRights::TRAVERSE));
        }

        assert_eq!(
            MachineRegistryLocation::Uninstall.components(),
            [
                "Software",
                "Microsoft",
                "Windows",
                "CurrentVersion",
                "Uninstall",
            ]
        );
        assert_eq!(
            MachineRegistryLocation::AppPaths.components(),
            [
                "Software",
                "Microsoft",
                "Windows",
                "CurrentVersion",
                "App Paths",
            ]
        );
    }

    #[test]
    fn any_leaf_or_intermediate_symbolic_link_marker_rejects_the_traversal() {
        let cases = [
            (
                ShellUserRegistryLocation::Environment,
                format!(r"HKEY_USERS\{SID}\Environment"),
            ),
            (
                ShellUserRegistryLocation::Run,
                format!(r"HKEY_USERS\{SID}\Software\Microsoft"),
            ),
        ];

        for (location, symbolic_path) in cases {
            let mut backend = FakeBackend::default();
            backend.symbolic_paths.insert(symbolic_path);

            let result = open_known_location(
                &mut backend,
                SID,
                location,
                RegistryRights::READ_VALUES,
                false,
            );

            assert_eq!(result, Err(RegistryTraversalError::SymbolicLinkComponent));
        }
    }

    #[test]
    fn newly_created_leaf_uses_only_the_returned_pinned_handle() {
        let mut backend = FakeBackend::default();
        let handle = open_known_location(
            &mut backend,
            SID,
            ShellUserRegistryLocation::Environment,
            RegistryRights::UPDATE_VALUES,
            true,
        )
        .expect("new Environment leaf should be usable");

        assert_eq!(handle, format!(r"HKEY_USERS\{SID}\Environment"));
        assert_eq!(
            backend
                .events
                .iter()
                .filter(|event| matches!(event, Event::Create { .. }))
                .count(),
            1
        );
        assert_eq!(
            backend
                .events
                .iter()
                .filter(|event| matches!(event, Event::Open { path, .. } if path.ends_with("Environment")))
                .count(),
            0
        );
    }

    #[test]
    fn existing_create_result_is_discarded_and_reopened_as_a_link() {
        let mut backend = FakeBackend {
            create_disposition: CreateDisposition::OpenedExisting,
            ..FakeBackend::default()
        };
        open_known_location(
            &mut backend,
            SID,
            ShellUserRegistryLocation::Environment,
            RegistryRights::UPDATE_VALUES,
            true,
        )
        .expect("normal existing Environment leaf should reopen");

        let environment_path = format!(r"HKEY_USERS\{SID}\Environment");
        let leaf_events: Vec<_> = backend
            .events
            .iter()
            .filter(|event| match event {
                Event::Create { path, .. } | Event::Open { path, .. } => path == &environment_path,
                Event::Inspect(_) => false,
            })
            .collect();
        assert!(matches!(leaf_events[0], Event::Create { .. }));
        assert!(matches!(leaf_events[1], Event::Open { .. }));
    }

    #[test]
    fn existing_create_result_cannot_hide_a_symbolic_link_leaf() {
        let environment_path = format!(r"HKEY_USERS\{SID}\Environment");
        let mut backend = FakeBackend {
            symbolic_paths: HashSet::from([environment_path]),
            create_disposition: CreateDisposition::OpenedExisting,
            ..FakeBackend::default()
        };

        let result = open_known_location(
            &mut backend,
            SID,
            ShellUserRegistryLocation::Environment,
            RegistryRights::UPDATE_VALUES,
            true,
        );

        assert_eq!(result, Err(RegistryTraversalError::SymbolicLinkComponent));
    }

    #[test]
    fn rights_are_minimal_and_create_permission_stays_on_the_parent() {
        let mut backend = FakeBackend::default();
        open_known_location(
            &mut backend,
            SID,
            ShellUserRegistryLocation::Environment,
            RegistryRights::UPDATE_VALUES,
            true,
        )
        .expect("Environment leaf should create");

        assert!(matches!(
            &backend.events[0],
            Event::Open { rights, .. }
                if *rights == RegistryRights::TRAVERSE.with_create_subkey()
        ));
        assert!(matches!(
            &backend.events[2],
            Event::Create { rights, .. } if *rights == RegistryRights::UPDATE_VALUES
        ));
    }

    #[test]
    fn missing_fixed_component_propagates_the_backend_error_and_stops_traversal() {
        let missing_path = format!(r"HKEY_USERS\{SID}\Software\Microsoft");
        let mut backend = FakeBackend::default();
        backend
            .open_errors
            .insert(missing_path.clone(), "missing component");

        let result = open_known_location(
            &mut backend,
            SID,
            ShellUserRegistryLocation::Run,
            RegistryRights::READ_VALUES,
            false,
        );

        assert_eq!(
            result,
            Err(RegistryTraversalError::Backend("missing component"))
        );
        assert!(matches!(
            backend.events.last(),
            Some(Event::Open { path, .. }) if path == &missing_path
        ));
        assert!(!backend.events.iter().any(|event| {
            matches!(event, Event::Open { path, .. } if path.ends_with(r"Microsoft\Windows"))
        }));
    }

    #[test]
    fn create_and_inspection_backend_errors_are_not_collapsed_into_security_errors() {
        let environment_path = format!(r"HKEY_USERS\{SID}\Environment");

        let mut create_failure = FakeBackend::default();
        create_failure
            .create_errors
            .insert(environment_path.clone(), "create failed");
        assert_eq!(
            open_known_location(
                &mut create_failure,
                SID,
                ShellUserRegistryLocation::Environment,
                RegistryRights::UPDATE_VALUES,
                true,
            ),
            Err(RegistryTraversalError::Backend("create failed"))
        );

        let mut inspection_failure = FakeBackend::default();
        inspection_failure
            .inspect_errors
            .insert(environment_path, "inspection failed");
        assert_eq!(
            open_known_location(
                &mut inspection_failure,
                SID,
                ShellUserRegistryLocation::Environment,
                RegistryRights::READ_VALUES,
                false,
            ),
            Err(RegistryTraversalError::Backend("inspection failed"))
        );
    }
}
