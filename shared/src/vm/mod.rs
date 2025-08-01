//! Virtual machine modules for running transactions, validity predicates,
//! matchmaker and matchmaker's filter.

use std::ffi::c_void;
use std::marker::PhantomData;
use std::slice;

use wasmparser::{Validator, WasmFeatures};

pub mod host_env;
pub mod memory;
pub mod prefix_iter;
pub mod types;
#[cfg(feature = "wasm-runtime")]
pub mod wasm;

/// This is used to attach the Ledger's host structures to wasm environment,
/// which is used for implementing some host calls. It wraps an immutable
/// reference, so the access is thread-safe, but because of the unsafe
/// reference conversion, care must be taken that while this reference is
/// borrowed, no other process can modify it.
#[derive(Clone)]
pub struct EnvHostWrapper<'a, T: 'a> {
    data: *const c_void,
    phantom: PhantomData<&'a T>,
}
unsafe impl<T> Send for EnvHostWrapper<'_, T> {}
unsafe impl<T> Sync for EnvHostWrapper<'_, T> {}

impl<'a, T: 'a> EnvHostWrapper<'a, &T> {
    /// Wrap a reference for VM environment.
    ///
    /// # Safety
    ///
    /// Because this is unsafe, care must be taken that while this reference
    /// is borrowed, no other process can modify it.
    pub unsafe fn new(host_structure: &T) -> Self {
        Self {
            data: host_structure as *const T as *const c_void,
            phantom: PhantomData,
        }
    }

    /// Get a reference from VM environment.
    ///
    /// # Safety
    ///
    /// Because this is unsafe, care must be taken that while this reference
    /// is borrowed, no other process can modify it.
    pub unsafe fn get(&self) -> &'a T {
        &*(self.data as *const T)
    }
}

/// This is used to attach the Ledger's host structures to wasm environment,
/// which is used for implementing some host calls. It wraps an immutable
/// slice, so the access is thread-safe, but because of the unsafe slice
/// conversion, care must be taken that while this slice is borrowed, no other
/// process can modify it.
#[derive(Clone)]
pub struct EnvHostSliceWrapper<'a, T: 'a> {
    data: *const c_void,
    len: usize,
    phantom: PhantomData<&'a T>,
}
unsafe impl<T> Send for EnvHostSliceWrapper<'_, T> {}
unsafe impl<T> Sync for EnvHostSliceWrapper<'_, T> {}

impl<'a, T: 'a> EnvHostSliceWrapper<'a, &[T]> {
    /// Wrap a slice for VM environment.
    ///
    /// # Safety
    ///
    /// Because this is unsafe, care must be taken that while this slice is
    /// borrowed, no other process can modify it.
    pub unsafe fn new(host_structure: &[T]) -> Self {
        Self {
            data: host_structure as *const [T] as *const c_void,
            len: host_structure.len(),
            phantom: PhantomData,
        }
    }

    /// Get a slice from VM environment.
    ///
    /// # Safety
    ///
    /// Because this is unsafe, care must be taken that while this slice is
    /// borrowed, no other process can modify it.
    pub unsafe fn get(&self) -> &'a [T] {
        slice::from_raw_parts(self.data as *const T, self.len)
    }
}

/// This is used to attach the Ledger's host structures to wasm environment,
/// which is used for implementing some host calls. Because it's mutable, it's
/// not thread-safe. Also, care must be taken that while this reference is
/// borrowed, no other process can read or modify it.
#[derive(Clone)]
pub struct MutEnvHostWrapper<'a, T: 'a> {
    data: *mut c_void,
    phantom: PhantomData<&'a T>,
}
unsafe impl<T> Send for MutEnvHostWrapper<'_, T> {}
unsafe impl<T> Sync for MutEnvHostWrapper<'_, T> {}

impl<'a, T: 'a> MutEnvHostWrapper<'a, &T> {
    /// Wrap a mutable reference for VM environment.
    ///
    /// # Safety
    ///
    /// This is not thread-safe. Also, because this is unsafe, care must be
    /// taken that while this reference is borrowed, no other process can read
    /// or modify it.
    pub unsafe fn new(host_structure: &mut T) -> Self {
        Self {
            data: host_structure as *mut T as *mut c_void,
            phantom: PhantomData,
        }
    }

    /// Get a mutable reference from VM environment.
    ///
    /// # Safety
    ///
    /// This is not thread-safe. Also, because this is unsafe, care must be
    /// taken that while this reference is borrowed, no other process can read
    /// or modify it.
    pub unsafe fn get(&self) -> &'a mut T {
        &mut *(self.data as *mut T)
    }
}

/// This is used to attach the Ledger's host structures to wasm environment,
/// which is used for implementing some host calls. It wraps an mutable
/// slice, so the access is thread-safe, but because of the unsafe slice
/// conversion, care must be taken that while this slice is borrowed, no other
/// process can modify it.
#[derive(Clone)]
pub struct MutEnvHostSliceWrapper<'a, T: 'a> {
    data: *mut c_void,
    len: usize,
    phantom: PhantomData<&'a T>,
}
unsafe impl<T> Send for MutEnvHostSliceWrapper<'_, T> {}
unsafe impl<T> Sync for MutEnvHostSliceWrapper<'_, T> {}

impl<'a, T: 'a> MutEnvHostSliceWrapper<'a, &[T]> {
    /// Wrap a slice for VM environment.
    ///
    /// # Safety
    ///
    /// Because this is unsafe, care must be taken that while this slice is
    /// borrowed, no other process can modify it.
    #[allow(dead_code)]
    pub unsafe fn new(host_structure: &mut [T]) -> Self {
        Self {
            data: host_structure as *mut [T] as *mut c_void,
            len: host_structure.len(),
            phantom: PhantomData,
        }
    }

    /// Get a slice from VM environment.
    ///
    /// # Safety
    ///
    /// Because this is unsafe, care must be taken that while this slice is
    /// borrowed, no other process can modify it.
    pub unsafe fn get(&self) -> &'a mut [T] {
        slice::from_raw_parts_mut(self.data as *mut T, self.len)
    }
}

/// Validate an untrusted wasm code with restrictions that we place such code
/// (e.g. transaction and validity predicates)
pub fn validate_untrusted_wasm(
    wasm_code: impl AsRef<[u8]>,
) -> Result<(), wasmparser::BinaryReaderError> {
    let mut validator = Validator::new();

    let features = WasmFeatures {
        reference_types: false,
        multi_value: false,
        bulk_memory: false,
        module_linking: false,
        simd: false,
        threads: false,
        tail_call: false,
        deterministic_only: true,
        multi_memory: false,
        exceptions: false,
        memory64: false,
    };
    validator.wasm_features(features);

    validator.validate_all(wasm_code.as_ref())
}
