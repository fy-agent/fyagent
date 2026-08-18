//! macOS Keychain leaf via raw Security.framework FFI.
//!
//! Frozen security-framework crates are intentionally not added. Dictionaries
//! match contract §10.1 exactly. Create is not upsert.

use std::ffi::c_void;
use std::ptr;
use std::sync::Mutex;

use crate::secret::{
    backend_material_callback_sealed, BackendDeleteDisposition, BackendDeleteMode,
    BackendMaterialWriteCallback, BackendRecordView, BackendRevocationObservationCapability,
    BackendVerifyReceiptId, BeginCaptureIntent, DeviceBindingGeneration, PendingConfirmationTermination,
    PlatformAuthorizedReadOutcome, PlatformBackendPort, PlatformDeleteResult, PlatformPrepareResult,
    PlatformProbeResult, PlatformWriteReadbackResult, SecretBackendGeneration,
    SecretBackendUnavailableReason, SecretInternalError, SecretLockSource, SecretMaterial,
    SecretOwner, SecretPurpose, SecretRecordCapabilities, SecretRef, SecretSourceFreeErrorCode,
    SecretTerminalOperationContext, UtcTimestamp,
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
const K_CF_NUMBER_SINT32_TYPE: i32 = 3;

type CFTypeRef = *const c_void;
type CFStringRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CFDataRef = *const c_void;
type CFBooleanRef = *const c_void;
type CFNumberRef = *const c_void;
type CFAllocatorRef = *const c_void;
type CFErrorRef = *mut c_void;
type CFIndex = isize;
type CFTypeID = usize;
type CFOptionFlags = usize;
type Boolean = u8;
type OSStatus = i32;
type SecAccessControlRef = *const c_void;

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

    fn CFRelease(cf: CFTypeRef);
    fn CFEqual(cf1: CFTypeRef, cf2: CFTypeRef) -> Boolean;
    fn CFGetTypeID(cf: CFTypeRef) -> CFTypeID;
    fn CFBooleanGetTypeID() -> CFTypeID;
    fn CFBooleanGetValue(boolean: CFBooleanRef) -> Boolean;
    fn CFNumberGetTypeID() -> CFTypeID;
    fn CFNumberGetValue(number: CFNumberRef, the_type: i32, value_ptr: *mut c_void) -> Boolean;
    fn CFStringCreateWithBytes(
        alloc: CFAllocatorRef,
        bytes: *const u8,
        num_bytes: CFIndex,
        encoding: u32,
        is_external: Boolean,
    ) -> CFStringRef;
    fn CFDataCreate(alloc: CFAllocatorRef, bytes: *const u8, length: CFIndex) -> CFDataRef;
    fn CFDataGetBytePtr(data: CFDataRef) -> *const u8;
    fn CFDataGetLength(data: CFDataRef) -> CFIndex;
    fn CFDictionaryCreate(
        alloc: CFAllocatorRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        num_values: CFIndex,
        key_callbacks: *const CFDictionaryKeyCallBacks,
        value_callbacks: *const CFDictionaryValueCallBacks,
    ) -> CFDictionaryRef;
    fn CFDictionaryGetValue(the_dict: CFDictionaryRef, key: CFTypeRef) -> CFTypeRef;
}

#[link(name = "Security", kind = "framework")]
extern "C" {
    static kSecClass: CFStringRef;
    static kSecClassGenericPassword: CFStringRef;
    static kSecAttrService: CFStringRef;
    static kSecAttrAccount: CFStringRef;
    static kSecAttrSynchronizable: CFStringRef;
    static kSecAttrAccessControl: CFStringRef;
    static kSecAttrAccessible: CFStringRef;
    static kSecAttrAccessibleWhenUnlockedThisDeviceOnly: CFStringRef;
    static kSecValueData: CFStringRef;
    static kSecReturnData: CFStringRef;
    static kSecReturnAttributes: CFStringRef;

    fn SecItemAdd(attributes: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
    fn SecItemCopyMatching(query: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
    fn SecItemUpdate(query: CFDictionaryRef, attributes_to_update: CFDictionaryRef) -> OSStatus;
    fn SecItemDelete(query: CFDictionaryRef) -> OSStatus;
    fn SecAccessControlCreateWithFlags(
        allocator: CFAllocatorRef,
        protection: CFTypeRef,
        flags: CFOptionFlags,
        error: *mut CFErrorRef,
    ) -> SecAccessControlRef;
}

struct CfObj(CFTypeRef);

impl CfObj {
    fn new(ptr: CFTypeRef) -> Result<Self, SecretInternalError> {
        if ptr.is_null() {
            Err(map_status(ERR_NOT_AVAILABLE, KeychainOp::Read))
        } else {
            Ok(Self(ptr))
        }
    }

    fn raw(&self) -> CFTypeRef {
        self.0
    }
}

impl Drop for CfObj {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) }
        }
    }
}

#[derive(Clone, Copy)]
enum KeychainOp {
    Create,
    Update,
    Read,
    Delete,
    Find,
}

fn capture_ctx() -> SecretTerminalOperationContext {
    SecretTerminalOperationContext::Capture(BeginCaptureIntent::NewBinding)
}

fn context_for(op: KeychainOp) -> SecretTerminalOperationContext {
    match op {
        KeychainOp::Create => capture_ctx(),
        KeychainOp::Delete => SecretTerminalOperationContext::Delete,
        KeychainOp::Update | KeychainOp::Read | KeychainOp::Find => {
            SecretTerminalOperationContext::Summary
        }
    }
}

fn map_status(status: OSStatus, op: KeychainOp) -> SecretInternalError {
    let context = context_for(op);
    match status {
        ERR_DUPLICATE_ITEM => SecretInternalError::terminal_operation_failure(
            SecretSourceFreeErrorCode::BackendChanged,
            context,
        ),
        ERR_ITEM_NOT_FOUND => SecretInternalError::terminal_operation_failure(
            SecretSourceFreeErrorCode::Missing,
            context,
        ),
        ERR_INTERACTION_NOT_ALLOWED | ERR_INTERACTION_REQUIRED => {
            SecretInternalError::locked(context, SecretLockSource::Backend)
        }
        ERR_AUTH_FAILED | ERR_MISSING_ENTITLEMENT => SecretInternalError::terminal_operation_failure(
            SecretSourceFreeErrorCode::PermissionDenied,
            context,
        ),
        ERR_NOT_AVAILABLE | ERR_NO_DEFAULT_KEYCHAIN | ERR_NO_STORAGE_MODULE => {
            SecretInternalError::backend_unavailable(
                context,
                SecretBackendUnavailableReason::OsStoreUnavailable,
            )
        }
        ERR_USER_CANCELED => SecretInternalError::terminal_operation_failure(
            SecretSourceFreeErrorCode::InputCancelled,
            context,
        ),
        ERR_DATA_TOO_LARGE | ERR_PARAM => SecretInternalError::terminal_operation_failure(
            SecretSourceFreeErrorCode::InputInvalid,
            context,
        ),
        _ => {
            let code = match op {
                KeychainOp::Create | KeychainOp::Update => SecretSourceFreeErrorCode::WriteFailed,
                KeychainOp::Delete => SecretSourceFreeErrorCode::DeleteFailed,
                KeychainOp::Read | KeychainOp::Find => SecretSourceFreeErrorCode::ReadFailed,
            };
            SecretInternalError::terminal_operation_failure(code, context)
        }
    }
}

fn cf_string(value: &str) -> Result<CfObj, SecretInternalError> {
    let ptr = unsafe {
        CFStringCreateWithBytes(
            ptr::null(),
            value.as_ptr(),
            value.len() as CFIndex,
            K_CF_STRING_ENCODING_UTF8,
            0,
        )
    };
    CfObj::new(ptr)
}

fn cf_data(bytes: &[u8]) -> Result<CfObj, SecretInternalError> {
    let ptr = unsafe { CFDataCreate(ptr::null(), bytes.as_ptr(), bytes.len() as CFIndex) };
    CfObj::new(ptr)
}

fn cf_dict(pairs: &[(CFTypeRef, CFTypeRef)]) -> Result<CfObj, SecretInternalError> {
    let keys: Vec<CFTypeRef> = pairs.iter().map(|pair| pair.0).collect();
    let values: Vec<CFTypeRef> = pairs.iter().map(|pair| pair.1).collect();
    let ptr = unsafe {
        CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            pairs.len() as CFIndex,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    CfObj::new(ptr)
}

fn cf_is_false(value: CFTypeRef) -> bool {
    if value.is_null() {
        return false;
    }
    unsafe {
        if CFGetTypeID(value) == CFBooleanGetTypeID() {
            CFBooleanGetValue(value as CFBooleanRef) == 0
        } else if CFGetTypeID(value) == CFNumberGetTypeID() {
            let mut number: i32 = 1;
            CFNumberGetValue(
                value as CFNumberRef,
                K_CF_NUMBER_SINT32_TYPE,
                &mut number as *mut i32 as *mut c_void,
            ) != 0
                && number == 0
        } else {
            false
        }
    }
}

fn cf_is_this_device_only(value: CFTypeRef) -> bool {
    if value.is_null() {
        return false;
    }
    unsafe { CFEqual(value, kSecAttrAccessibleWhenUnlockedThisDeviceOnly as CFTypeRef) != 0 }
}

fn identity_query(service: CFTypeRef, account: CFTypeRef) -> Result<CfObj, SecretInternalError> {
    unsafe {
        cf_dict(&[
            (kSecClass as CFTypeRef, kSecClassGenericPassword as CFTypeRef),
            (kSecAttrService as CFTypeRef, service),
            (kSecAttrAccount as CFTypeRef, account),
            (kSecAttrSynchronizable as CFTypeRef, kCFBooleanFalse as CFTypeRef),
        ])
    }
}

fn read_query(service: CFTypeRef, account: CFTypeRef) -> Result<CfObj, SecretInternalError> {
    unsafe {
        cf_dict(&[
            (kSecClass as CFTypeRef, kSecClassGenericPassword as CFTypeRef),
            (kSecAttrService as CFTypeRef, service),
            (kSecAttrAccount as CFTypeRef, account),
            (kSecAttrSynchronizable as CFTypeRef, kCFBooleanFalse as CFTypeRef),
            (kSecReturnData as CFTypeRef, kCFBooleanTrue as CFTypeRef),
        ])
    }
}

fn attributes_query(service: CFTypeRef, account: CFTypeRef) -> Result<CfObj, SecretInternalError> {
    unsafe {
        cf_dict(&[
            (kSecClass as CFTypeRef, kSecClassGenericPassword as CFTypeRef),
            (kSecAttrService as CFTypeRef, service),
            (kSecAttrAccount as CFTypeRef, account),
            (kSecAttrSynchronizable as CFTypeRef, kCFBooleanFalse as CFTypeRef),
            (kSecReturnAttributes as CFTypeRef, kCFBooleanTrue as CFTypeRef),
        ])
    }
}

fn copy_matching(query: CFTypeRef) -> Result<CfObj, OSStatus> {
    let mut result: CFTypeRef = ptr::null();
    let status = unsafe { SecItemCopyMatching(query as CFDictionaryRef, &mut result) };
    if status != ERR_SUCCESS {
        if !result.is_null() {
            unsafe { CFRelease(result) }
        }
        return Err(status);
    }
    CfObj::new(result).map_err(|_| ERR_NOT_AVAILABLE)
}

fn data_to_bytes(data: CFTypeRef) -> Result<Vec<u8>, SecretInternalError> {
    if data.is_null() {
        return Err(SecretInternalError::input_invalid());
    }
    unsafe {
        let len = CFDataGetLength(data as CFDataRef);
        if len < 0 {
            return Err(SecretInternalError::input_invalid());
        }
        let ptr = CFDataGetBytePtr(data as CFDataRef);
        if ptr.is_null() && len > 0 {
            return Err(SecretInternalError::input_invalid());
        }
        Ok(std::slice::from_raw_parts(ptr, len as usize).to_vec())
    }
}

fn assert_required_attributes(
    attrs: CFTypeRef,
    context: SecretTerminalOperationContext,
) -> Result<(), SecretInternalError> {
    if attrs.is_null() {
        return Err(SecretInternalError::terminal_operation_failure(
            SecretSourceFreeErrorCode::VerifyFailed,
            context,
        ));
    }
    let sync = unsafe { CFDictionaryGetValue(attrs as CFDictionaryRef, kSecAttrSynchronizable as CFTypeRef) };
    let accessible =
        unsafe { CFDictionaryGetValue(attrs as CFDictionaryRef, kSecAttrAccessible as CFTypeRef) };
    if cf_is_false(sync) && cf_is_this_device_only(accessible) {
        Ok(())
    } else {
        Err(SecretInternalError::terminal_operation_failure(
            SecretSourceFreeErrorCode::VerifyFailed,
            context,
        ))
    }
}

fn read_bytes(
    service: CFTypeRef,
    account: CFTypeRef,
) -> Result<Vec<u8>, SecretInternalError> {
    let query = read_query(service, account)?;
    match copy_matching(query.raw()) {
        Ok(data) => data_to_bytes(data.raw()),
        Err(status) => Err(map_status(status, KeychainOp::Read)),
    }
}

fn read_and_verify_attributes(
    service: CFTypeRef,
    account: CFTypeRef,
    expected: &[u8],
    op: KeychainOp,
) -> Result<SecretMaterial, SecretInternalError> {
    let bytes = read_bytes(service, account)?;
    let material = SecretMaterial::from_native_input(bytes, SecretPurpose::CodexApiKey)?;
    if !material.ct_eq_slice(expected) {
        return Err(SecretInternalError::terminal_operation_failure(
            SecretSourceFreeErrorCode::VerifyFailed,
            context_for(op),
        ));
    }
    let query = attributes_query(service, account)?;
    match copy_matching(query.raw()) {
        Ok(attrs) => assert_required_attributes(attrs.raw(), context_for(op))?,
        Err(status) => return Err(map_status(status, KeychainOp::Read)),
    }
    Ok(material)
}

fn create_access_control() -> Result<CfObj, SecretInternalError> {
    let mut error: CFErrorRef = ptr::null_mut();
    let access = unsafe {
        SecAccessControlCreateWithFlags(
            ptr::null(),
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly as CFTypeRef,
            0,
            &mut error,
        )
    };
    if !error.is_null() {
        unsafe { CFRelease(error as CFTypeRef) }
    }
    if access.is_null() {
        Err(map_status(ERR_NOT_AVAILABLE, KeychainOp::Create))
    } else {
        Ok(CfObj(access as CFTypeRef))
    }
}

pub(crate) struct MacOsLeafReceipt {
    pub(crate) backend_generation: SecretBackendGeneration,
    pub(crate) device_binding_generation: DeviceBindingGeneration,
}

pub(crate) struct MacOsSecretStore {
    lock: Mutex<()>,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
}

impl MacOsSecretStore {
    pub(crate) fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            backend_generation: SecretBackendGeneration::parse(1).expect("generation 1"),
            device_binding_generation: DeviceBindingGeneration::parse(1).expect("generation 1"),
        }
    }

    fn generations(&self) -> MacOsLeafReceipt {
        MacOsLeafReceipt {
            backend_generation: self.backend_generation,
            device_binding_generation: self.device_binding_generation,
        }
    }

    fn lock_instance(&self) -> Result<std::sync::MutexGuard<'_, ()>, SecretInternalError> {
        self.lock
            .lock()
            .map_err(|_| SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::Internal,
                SecretTerminalOperationContext::Summary,
            ))
    }

    pub(crate) fn create_new(
        &self,
        secret_ref: &SecretRef,
        material: SecretMaterial,
    ) -> Result<MacOsLeafReceipt, SecretInternalError> {
        let mut outcome = Err(SecretInternalError::input_invalid());
        material.write_to_sealed_callback(LeafWriteCallback {
            store: self,
            secret_ref,
            mode: LeafWriteMode::Create,
            outcome: &mut outcome,
        });
        outcome
    }

    pub(crate) fn replace(
        &self,
        secret_ref: &SecretRef,
        material: SecretMaterial,
    ) -> Result<MacOsLeafReceipt, SecretInternalError> {
        let mut outcome = Err(SecretInternalError::input_invalid());
        material.write_to_sealed_callback(LeafWriteCallback {
            store: self,
            secret_ref,
            mode: LeafWriteMode::Replace,
            outcome: &mut outcome,
        });
        outcome
    }

    pub(crate) fn read(&self, secret_ref: &SecretRef) -> Result<SecretMaterial, SecretInternalError> {
        let _guard = self.lock_instance()?;
        let service = cf_string(SERVICE)?;
        let account = cf_string(secret_ref.as_str())?;
        let bytes = read_bytes(service.raw(), account.raw())?;
        SecretMaterial::from_native_input(bytes, SecretPurpose::CodexApiKey)
    }

    pub(crate) fn delete(&self, secret_ref: &SecretRef) -> Result<MacOsLeafReceipt, SecretInternalError> {
        let _guard = self.lock_instance()?;
        self.delete_locked(secret_ref, false)?;
        Ok(self.generations())
    }

    pub(crate) fn validate_missing(
        &self,
        secret_ref: &SecretRef,
    ) -> Result<(), SecretInternalError> {
        let _guard = self.lock_instance()?;
        match self.find_locked(secret_ref) {
            Ok(true) => Err(SecretInternalError::input_invalid()),
            Ok(false) => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn create_bytes(
        &self,
        secret_ref: &SecretRef,
        material: &[u8],
    ) -> Result<MacOsLeafReceipt, SecretInternalError> {
        let context = capture_ctx();
        let service = cf_string(SERVICE)?;
        let account = cf_string(secret_ref.as_str())?;
        let data = cf_data(material)?;
        let access = create_access_control()?;
        // Create-only 6-key dictionary. No kSecAttrAccessible, label, auth
        // context, return-data/ref, or sync-any.
        let attributes = unsafe {
            cf_dict(&[
                (kSecClass as CFTypeRef, kSecClassGenericPassword as CFTypeRef),
                (kSecAttrService as CFTypeRef, service.raw()),
                (kSecAttrAccount as CFTypeRef, account.raw()),
                (kSecAttrSynchronizable as CFTypeRef, kCFBooleanFalse as CFTypeRef),
                (kSecAttrAccessControl as CFTypeRef, access.raw()),
                (kSecValueData as CFTypeRef, data.raw()),
            ])?
        };
        let status = unsafe { SecItemAdd(attributes.raw() as CFDictionaryRef, ptr::null_mut()) };
        if status == ERR_DUPLICATE_ITEM {
            // NEVER SecItemUpdate on duplicate create.
            return Err(map_status(status, KeychainOp::Create));
        }
        if status != ERR_SUCCESS {
            return Err(map_status(status, KeychainOp::Create));
        }
        let _readback = read_and_verify_attributes(service.raw(), account.raw(), material, KeychainOp::Create)?;
        Ok(self.generations())
    }

    fn replace_bytes(
        &self,
        secret_ref: &SecretRef,
        material: &[u8],
    ) -> Result<MacOsLeafReceipt, SecretInternalError> {
        let context = SecretTerminalOperationContext::Summary;
        let service = cf_string(SERVICE)?;
        let account = cf_string(secret_ref.as_str())?;
        match self.find_with(service.raw(), account.raw()) {
            Ok(true) => {}
            Ok(false) => {
                return Err(SecretInternalError::terminal_operation_failure(
                    SecretSourceFreeErrorCode::Missing,
                    context,
                ))
            }
            Err(error) => return Err(error),
        }
        let data = cf_data(material)?;
        let query = identity_query(service.raw(), account.raw())?;
        let update = unsafe { cf_dict(&[(kSecValueData as CFTypeRef, data.raw())])? };
        let status = unsafe {
            SecItemUpdate(
                query.raw() as CFDictionaryRef,
                update.raw() as CFDictionaryRef,
            )
        };
        if status == ERR_ITEM_NOT_FOUND {
            // not-found on replace must NOT fall into create.
            return Err(map_status(status, KeychainOp::Update));
        }
        if status != ERR_SUCCESS {
            return Err(map_status(status, KeychainOp::Update));
        }
        let _readback = read_and_verify_attributes(service.raw(), account.raw(), material, KeychainOp::Update)?;
        Ok(self.generations())
    }

    fn find_locked(&self, secret_ref: &SecretRef) -> Result<bool, SecretInternalError> {
        let service = cf_string(SERVICE)?;
        let account = cf_string(secret_ref.as_str())?;
        self.find_with(service.raw(), account.raw())
    }

    fn find_with(
        &self,
        service: CFTypeRef,
        account: CFTypeRef,
    ) -> Result<bool, SecretInternalError> {
        let query = attributes_query(service, account)?;
        match copy_matching(query.raw()) {
            Ok(_) => Ok(true),
            Err(ERR_ITEM_NOT_FOUND) => Ok(false),
            Err(status) => Err(map_status(status, KeychainOp::Find)),
        }
    }

    fn delete_locked(
        &self,
        secret_ref: &SecretRef,
        already_missing_ok: bool,
    ) -> Result<BackendDeleteDisposition, SecretInternalError> {
        let context = SecretTerminalOperationContext::Delete;
        let service = cf_string(SERVICE)?;
        let account = cf_string(secret_ref.as_str())?;
        let query = identity_query(service.raw(), account.raw())?;
        let status = unsafe { SecItemDelete(query.raw() as CFDictionaryRef) };
        match status {
            ERR_SUCCESS => Ok(BackendDeleteDisposition::Deleted),
            ERR_ITEM_NOT_FOUND if already_missing_ok => {
                Ok(BackendDeleteDisposition::AlreadyMissing)
            }
            other => Err(map_status(other, KeychainOp::Delete)),
        }
    }

    fn now_timestamp() -> Result<UtcTimestamp, SecretInternalError> {
        UtcTimestamp::parse(crate::secret::device_store::utc_now())
            .map_err(|_| SecretInternalError::input_invalid())
    }
}

enum LeafWriteMode {
    Create,
    Replace,
}

struct LeafWriteCallback<'a> {
    store: &'a MacOsSecretStore,
    secret_ref: &'a SecretRef,
    mode: LeafWriteMode,
    outcome: &'a mut Result<MacOsLeafReceipt, SecretInternalError>,
}

impl backend_material_callback_sealed::Sealed for LeafWriteCallback<'_> {}

impl BackendMaterialWriteCallback for LeafWriteCallback<'_> {
    type Receipt = ();

    fn write_once(self, material: &[u8]) -> Self::Receipt {
        *self.outcome = match self.mode {
            LeafWriteMode::Create => self.store.create_bytes(self.secret_ref, material),
            LeafWriteMode::Replace => self.store.replace_bytes(self.secret_ref, material),
        };
    }
}

impl PlatformBackendPort for MacOsSecretStore {
    fn revocation_observation_capability(&self) -> BackendRevocationObservationCapability {
        BackendRevocationObservationCapability::Unsupported
    }

    fn capabilities_for_record(
        &self,
        _record: BackendRecordView<'_>,
        _purpose: SecretPurpose,
    ) -> Result<SecretRecordCapabilities, SecretInternalError> {
        Err(SecretInternalError::input_invalid())
    }

    fn capabilities_for_new_record(
        &self,
        _owner: &SecretOwner,
        _purpose: SecretPurpose,
    ) -> Result<SecretRecordCapabilities, SecretInternalError> {
        Err(SecretInternalError::input_invalid())
    }

    fn prepare(
        &self,
        _record: BackendRecordView<'_>,
        _requirement: crate::secret::PlatformOperationRequirement<'_>,
    ) -> Result<PlatformPrepareResult, SecretInternalError> {
        Ok(PlatformPrepareResult::Ready {
            authorization_id: 1,
        })
    }

    fn confirm(&self, _pending_id: u128) -> Result<u128, SecretInternalError> {
        Err(SecretInternalError::terminal_operation_failure(
            SecretSourceFreeErrorCode::ConfirmationCancelled,
            SecretTerminalOperationContext::Validation,
        ))
    }

    fn cancel(
        &self,
        _pending_id: u128,
        _reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        Ok(())
    }

    fn write_and_readback_bytes(
        &self,
        record: BackendRecordView<'_>,
        _authorization_id: u128,
        material: &[u8],
    ) -> Result<PlatformWriteReadbackResult, SecretInternalError> {
        let _guard = self.lock_instance()?;
        let receipt = self.replace_bytes(record.secret_ref(), material)?;
        let readback = {
            let service = cf_string(SERVICE)?;
            let account = cf_string(record.secret_ref().as_str())?;
            let bytes = read_bytes(service.raw(), account.raw())?;
            SecretMaterial::from_native_input(bytes, SecretPurpose::CodexApiKey)?
        };
        Ok(PlatformWriteReadbackResult {
            readback,
            verify_receipt_id: BackendVerifyReceiptId(uuid::Uuid::new_v4().into_bytes()),
            backend_generation: receipt.backend_generation,
            device_binding_generation: receipt.device_binding_generation,
        })
    }

    fn read_authorized_material_once(
        &self,
        record: BackendRecordView<'_>,
        _authorization_id: u128,
    ) -> Result<PlatformAuthorizedReadOutcome, SecretInternalError> {
        let material = self.read(record.secret_ref())?;
        Ok(PlatformAuthorizedReadOutcome::Material {
            material,
            backend_generation: self.backend_generation,
            device_binding_generation: self.device_binding_generation,
        })
    }

    fn probe(
        &self,
        record: BackendRecordView<'_>,
    ) -> Result<PlatformProbeResult, SecretInternalError> {
        let _guard = self.lock_instance()?;
        // OS keyring has no central revocation; missing is missing, never revoked.
        match self.find_locked(record.secret_ref())? {
            true => Ok(PlatformProbeResult::Present {
                backend_generation: self.backend_generation,
                device_binding_generation: self.device_binding_generation,
            }),
            false => Ok(PlatformProbeResult::Missing {
                backend_generation: self.backend_generation,
                device_binding_generation: self.device_binding_generation,
            }),
        }
    }

    fn observe_revocation_once(
        &self,
        _record: BackendRecordView<'_>,
        _authorization_id: u128,
    ) -> Result<crate::secret::PlatformRevocationObservationResult, SecretInternalError> {
        // Must not persist revoked from probe/missing. Observation is unsupported.
        Err(SecretInternalError::terminal_operation_failure(
            SecretSourceFreeErrorCode::PermissionDenied,
            SecretTerminalOperationContext::Delete,
        ))
    }

    fn delete_or_revoke(
        &self,
        record: BackendRecordView<'_>,
        _authorization_id: u128,
        mode: BackendDeleteMode,
    ) -> Result<PlatformDeleteResult, SecretInternalError> {
        if !matches!(mode, BackendDeleteMode::Delete) {
            return Err(SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::PermissionDenied,
                SecretTerminalOperationContext::Delete,
            ));
        }
        let _guard = self.lock_instance()?;
        let disposition = self.delete_locked(record.secret_ref(), true)?;
        Ok(PlatformDeleteResult {
            disposition,
            completed_at: Self::now_timestamp()?,
            backend_generation: self.backend_generation,
            device_binding_generation: self.device_binding_generation,
        })
    }
}
