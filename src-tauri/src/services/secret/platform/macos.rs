#![allow(non_camel_case_types, non_upper_case_globals)]

use std::{ffi::c_void, ptr, sync::Mutex};

use super::super::{
    BackendProbe, SecretBackend, SecretBackendKind, SecretMaterial, SecretPurpose, SecretRef,
    SecretServiceError,
};

const SERVICE: &str = "com.fyagent.secrets.v1";

const ERR_SUCCESS: i32 = 0;
const ERR_USER_CANCELED: i32 = -128;
const ERR_PARAM: i32 = -50;
const ERR_NOT_AVAILABLE: i32 = -25291;
const ERR_AUTH_FAILED: i32 = -25293;
const ERR_DUPLICATE_ITEM: i32 = -25299;
const ERR_ITEM_NOT_FOUND: i32 = -25300;
const ERR_DATA_TOO_LARGE: i32 = -25302;
const ERR_NO_DEFAULT_KEYCHAIN: i32 = -25307;
const ERR_INTERACTION_NOT_ALLOWED: i32 = -25308;
const ERR_NO_STORAGE_MODULE: i32 = -25312;
const ERR_INTERACTION_REQUIRED: i32 = -25315;
const ERR_MISSING_ENTITLEMENT: i32 = -34018;

const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

type CFTypeRef = *const c_void;
type CFStringRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CFDataRef = *const c_void;
type CFBooleanRef = *const c_void;
type CFAllocatorRef = *const c_void;
type CFIndex = isize;
type Boolean = u8;
type OSStatus = i32;

#[repr(C)]
struct CFDictionaryKeyCallBacks {
    version: CFIndex,
    retain: *const c_void,
    release: *const c_void,
    copy_description: *const c_void,
    equal: *const c_void,
    hash: *const c_void,
}

#[repr(C)]
struct CFDictionaryValueCallBacks {
    version: CFIndex,
    retain: *const c_void,
    release: *const c_void,
    copy_description: *const c_void,
    equal: *const c_void,
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFBooleanTrue: CFBooleanRef;
    static kCFBooleanFalse: CFBooleanRef;
    static kCFTypeDictionaryKeyCallBacks: CFDictionaryKeyCallBacks;
    static kCFTypeDictionaryValueCallBacks: CFDictionaryValueCallBacks;

    fn CFRelease(value: CFTypeRef);
    fn CFStringCreateWithBytes(
        allocator: CFAllocatorRef,
        bytes: *const u8,
        count: CFIndex,
        encoding: u32,
        is_external_representation: Boolean,
    ) -> CFStringRef;
    fn CFDataCreate(allocator: CFAllocatorRef, bytes: *const u8, count: CFIndex) -> CFDataRef;
    fn CFDataGetBytePtr(data: CFDataRef) -> *const u8;
    fn CFDataGetLength(data: CFDataRef) -> CFIndex;
    fn CFDictionaryCreate(
        allocator: CFAllocatorRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        count: CFIndex,
        key_callbacks: *const CFDictionaryKeyCallBacks,
        value_callbacks: *const CFDictionaryValueCallBacks,
    ) -> CFDictionaryRef;
}

#[link(name = "Security", kind = "framework")]
extern "C" {
    static kSecClass: CFStringRef;
    static kSecClassGenericPassword: CFStringRef;
    static kSecAttrService: CFStringRef;
    static kSecAttrAccount: CFStringRef;
    static kSecAttrSynchronizable: CFStringRef;
    static kSecAttrAccessible: CFStringRef;
    static kSecAttrAccessibleWhenUnlockedThisDeviceOnly: CFStringRef;
    static kSecUseDataProtectionKeychain: CFStringRef;
    static kSecValueData: CFStringRef;
    static kSecReturnData: CFStringRef;
    static kSecReturnAttributes: CFStringRef;
    static kSecMatchLimit: CFStringRef;
    static kSecMatchLimitOne: CFStringRef;

    fn SecItemAdd(attributes: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
    fn SecItemCopyMatching(query: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
    fn SecItemUpdate(query: CFDictionaryRef, attributes: CFDictionaryRef) -> OSStatus;
    fn SecItemDelete(query: CFDictionaryRef) -> OSStatus;
}

struct CfOwned(CFTypeRef);

impl CfOwned {
    fn new(value: CFTypeRef) -> Result<Self, SecretServiceError> {
        if value.is_null() {
            Err(SecretServiceError::backend_unavailable())
        } else {
            Ok(Self(value))
        }
    }

    fn raw(&self) -> CFTypeRef {
        self.0
    }
}

impl Drop for CfOwned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) };
        }
    }
}

#[derive(Clone, Copy)]
enum KeychainOperation {
    Create,
    Replace,
    Read,
    Probe,
    Delete,
}

fn map_status(status: OSStatus, operation: KeychainOperation) -> SecretServiceError {
    match status {
        ERR_DUPLICATE_ITEM => SecretServiceError::already_exists(),
        ERR_ITEM_NOT_FOUND => SecretServiceError::missing(),
        ERR_INTERACTION_NOT_ALLOWED | ERR_INTERACTION_REQUIRED => SecretServiceError::locked(),
        ERR_AUTH_FAILED | ERR_MISSING_ENTITLEMENT => SecretServiceError::permission_denied(),
        ERR_NOT_AVAILABLE | ERR_NO_DEFAULT_KEYCHAIN | ERR_NO_STORAGE_MODULE => {
            SecretServiceError::backend_unavailable()
        }
        ERR_USER_CANCELED | ERR_DATA_TOO_LARGE | ERR_PARAM => SecretServiceError::invalid_input(),
        _ => match operation {
            KeychainOperation::Create | KeychainOperation::Replace => {
                SecretServiceError::write_failed()
            }
            KeychainOperation::Read | KeychainOperation::Probe => SecretServiceError::read_failed(),
            KeychainOperation::Delete => SecretServiceError::delete_failed(),
        },
    }
}

fn cf_string(value: &str) -> Result<CfOwned, SecretServiceError> {
    let raw = unsafe {
        CFStringCreateWithBytes(
            ptr::null(),
            value.as_ptr(),
            value.len() as CFIndex,
            K_CF_STRING_ENCODING_UTF8,
            0,
        )
    };
    CfOwned::new(raw)
}

fn cf_data(value: &[u8]) -> Result<CfOwned, SecretServiceError> {
    let raw = unsafe { CFDataCreate(ptr::null(), value.as_ptr(), value.len() as CFIndex) };
    CfOwned::new(raw)
}

fn cf_dictionary(pairs: &[(CFTypeRef, CFTypeRef)]) -> Result<CfOwned, SecretServiceError> {
    let keys: Vec<_> = pairs.iter().map(|(key, _)| *key).collect();
    let values: Vec<_> = pairs.iter().map(|(_, value)| *value).collect();
    let raw = unsafe {
        CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            pairs.len() as CFIndex,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    CfOwned::new(raw)
}

fn identity_query(service: CFTypeRef, account: CFTypeRef) -> Result<CfOwned, SecretServiceError> {
    unsafe {
        cf_dictionary(&[
            (
                kSecClass as CFTypeRef,
                kSecClassGenericPassword as CFTypeRef,
            ),
            (kSecAttrService as CFTypeRef, service),
            (kSecAttrAccount as CFTypeRef, account),
            (
                kSecAttrSynchronizable as CFTypeRef,
                kCFBooleanFalse as CFTypeRef,
            ),
            (
                kSecUseDataProtectionKeychain as CFTypeRef,
                kCFBooleanTrue as CFTypeRef,
            ),
            (kSecMatchLimit as CFTypeRef, kSecMatchLimitOne as CFTypeRef),
        ])
    }
}

fn read_query(service: CFTypeRef, account: CFTypeRef) -> Result<CfOwned, SecretServiceError> {
    unsafe {
        cf_dictionary(&[
            (
                kSecClass as CFTypeRef,
                kSecClassGenericPassword as CFTypeRef,
            ),
            (kSecAttrService as CFTypeRef, service),
            (kSecAttrAccount as CFTypeRef, account),
            (
                kSecAttrSynchronizable as CFTypeRef,
                kCFBooleanFalse as CFTypeRef,
            ),
            (
                kSecUseDataProtectionKeychain as CFTypeRef,
                kCFBooleanTrue as CFTypeRef,
            ),
            (kSecMatchLimit as CFTypeRef, kSecMatchLimitOne as CFTypeRef),
            (kSecReturnData as CFTypeRef, kCFBooleanTrue as CFTypeRef),
        ])
    }
}

fn probe_query(service: CFTypeRef, account: CFTypeRef) -> Result<CfOwned, SecretServiceError> {
    unsafe {
        cf_dictionary(&[
            (
                kSecClass as CFTypeRef,
                kSecClassGenericPassword as CFTypeRef,
            ),
            (kSecAttrService as CFTypeRef, service),
            (kSecAttrAccount as CFTypeRef, account),
            (
                kSecAttrSynchronizable as CFTypeRef,
                kCFBooleanFalse as CFTypeRef,
            ),
            (
                kSecUseDataProtectionKeychain as CFTypeRef,
                kCFBooleanTrue as CFTypeRef,
            ),
            (kSecMatchLimit as CFTypeRef, kSecMatchLimitOne as CFTypeRef),
            (
                kSecReturnAttributes as CFTypeRef,
                kCFBooleanTrue as CFTypeRef,
            ),
        ])
    }
}

fn copy_matching(query: &CfOwned) -> Result<CfOwned, OSStatus> {
    let mut result = ptr::null();
    let status = unsafe { SecItemCopyMatching(query.raw() as CFDictionaryRef, &mut result) };
    if status != ERR_SUCCESS {
        if !result.is_null() {
            unsafe { CFRelease(result) };
        }
        return Err(status);
    }
    if result.is_null() {
        Err(ERR_NOT_AVAILABLE)
    } else {
        Ok(CfOwned(result))
    }
}

fn copy_data(data: CFTypeRef) -> Result<Vec<u8>, SecretServiceError> {
    let length = unsafe { CFDataGetLength(data as CFDataRef) };
    if length <= 0 || length > 2_560 {
        return Err(SecretServiceError::read_failed());
    }
    let bytes = unsafe { CFDataGetBytePtr(data as CFDataRef) };
    if bytes.is_null() {
        return Err(SecretServiceError::read_failed());
    }
    Ok(unsafe { std::slice::from_raw_parts(bytes, length as usize) }.to_vec())
}

pub(crate) struct MacOsSecretBackend {
    lock: Mutex<()>,
}

impl MacOsSecretBackend {
    pub(crate) fn new() -> Self {
        Self {
            lock: Mutex::new(()),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, ()>, SecretServiceError> {
        self.lock.lock().map_err(|_| SecretServiceError::internal())
    }

    fn read_locked(&self, secret_ref: &SecretRef) -> Result<SecretMaterial, SecretServiceError> {
        let service = cf_string(SERVICE)?;
        let account = cf_string(secret_ref.as_str())?;
        let query = read_query(service.raw(), account.raw())?;
        let result =
            copy_matching(&query).map_err(|status| map_status(status, KeychainOperation::Read))?;
        SecretMaterial::from_native_input(copy_data(result.raw())?, SecretPurpose::CodexApiKey)
    }

    fn verify_locked(
        &self,
        secret_ref: &SecretRef,
        expected: &SecretMaterial,
    ) -> Result<(), SecretServiceError> {
        let actual = self.read_locked(secret_ref)?;
        if actual.ct_eq_slice(expected.as_bytes()) {
            Ok(())
        } else {
            Err(SecretServiceError::verify_failed())
        }
    }
}

impl SecretBackend for MacOsSecretBackend {
    fn kind(&self) -> SecretBackendKind {
        SecretBackendKind::OsKeyring
    }

    fn create_new(
        &self,
        secret_ref: &SecretRef,
        material: &SecretMaterial,
    ) -> Result<(), SecretServiceError> {
        let _guard = self.lock()?;
        let service = cf_string(SERVICE)?;
        let account = cf_string(secret_ref.as_str())?;
        let data = cf_data(material.as_bytes())?;
        let attributes = unsafe {
            cf_dictionary(&[
                (
                    kSecClass as CFTypeRef,
                    kSecClassGenericPassword as CFTypeRef,
                ),
                (kSecAttrService as CFTypeRef, service.raw()),
                (kSecAttrAccount as CFTypeRef, account.raw()),
                (
                    kSecAttrSynchronizable as CFTypeRef,
                    kCFBooleanFalse as CFTypeRef,
                ),
                (
                    kSecUseDataProtectionKeychain as CFTypeRef,
                    kCFBooleanTrue as CFTypeRef,
                ),
                (
                    kSecAttrAccessible as CFTypeRef,
                    kSecAttrAccessibleWhenUnlockedThisDeviceOnly as CFTypeRef,
                ),
                (kSecValueData as CFTypeRef, data.raw()),
            ])?
        };
        let status = unsafe { SecItemAdd(attributes.raw() as CFDictionaryRef, ptr::null_mut()) };
        if status != ERR_SUCCESS {
            return Err(map_status(status, KeychainOperation::Create));
        }
        if let Err(error) = self.verify_locked(secret_ref, material) {
            if let Ok(query) = identity_query(service.raw(), account.raw()) {
                unsafe { SecItemDelete(query.raw() as CFDictionaryRef) };
            }
            return Err(error);
        }
        Ok(())
    }

    fn replace(
        &self,
        secret_ref: &SecretRef,
        material: &SecretMaterial,
    ) -> Result<(), SecretServiceError> {
        let _guard = self.lock()?;
        let service = cf_string(SERVICE)?;
        let account = cf_string(secret_ref.as_str())?;
        let query = identity_query(service.raw(), account.raw())?;
        let data = cf_data(material.as_bytes())?;
        let updates = unsafe { cf_dictionary(&[(kSecValueData as CFTypeRef, data.raw())])? };
        let status = unsafe {
            SecItemUpdate(
                query.raw() as CFDictionaryRef,
                updates.raw() as CFDictionaryRef,
            )
        };
        if status != ERR_SUCCESS {
            return Err(map_status(status, KeychainOperation::Replace));
        }
        self.verify_locked(secret_ref, material)
    }

    fn read(&self, secret_ref: &SecretRef) -> Result<SecretMaterial, SecretServiceError> {
        let _guard = self.lock()?;
        self.read_locked(secret_ref)
    }

    fn probe(&self, secret_ref: &SecretRef) -> Result<BackendProbe, SecretServiceError> {
        let _guard = self.lock()?;
        let service = cf_string(SERVICE)?;
        let account = cf_string(secret_ref.as_str())?;
        let query = probe_query(service.raw(), account.raw())?;
        match copy_matching(&query) {
            Ok(_) => Ok(BackendProbe::ready()),
            Err(ERR_ITEM_NOT_FOUND) => Ok(BackendProbe::missing()),
            Err(status) => Err(map_status(status, KeychainOperation::Probe)),
        }
    }

    fn delete(&self, secret_ref: &SecretRef) -> Result<(), SecretServiceError> {
        let _guard = self.lock()?;
        let service = cf_string(SERVICE)?;
        let account = cf_string(secret_ref.as_str())?;
        let query = identity_query(service.raw(), account.raw())?;
        let status = unsafe { SecItemDelete(query.raw() as CFDictionaryRef) };
        if status == ERR_SUCCESS {
            Ok(())
        } else {
            Err(map_status(status, KeychainOperation::Delete))
        }
    }
}
