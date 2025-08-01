/// Transaction environment imports
pub mod tx {
    pub use core::slice;
    use std::convert::TryFrom;
    use std::marker::PhantomData;
    pub use std::mem::size_of;

    use anoma_shared::types::internal::HostEnvResult;
    use anoma_shared::types::{
        Address, BlockHash, BlockHeight, BLOCK_HASH_LENGTH, CHAIN_ID_LENGTH,
    };
    use anoma_shared::vm::types::KeyVal;
    pub use borsh::{BorshDeserialize, BorshSerialize};

    pub struct KeyValIterator<T>(pub u64, pub PhantomData<T>);

    impl<T: BorshDeserialize> Iterator for KeyValIterator<T> {
        type Item = (String, T);

        fn next(&mut self) -> Option<(String, T)> {
            let result: Vec<u8> = Vec::with_capacity(0);
            let size =
                unsafe { anoma_tx_iter_next(self.0, result.as_ptr() as _) };
            if HostEnvResult::is_fail(size) {
                None
            } else {
                let slice = unsafe {
                    slice::from_raw_parts(result.as_ptr(), size as _)
                };
                match KeyVal::try_from_slice(slice) {
                    Ok(key_val) => match T::try_from_slice(&key_val.val) {
                        Ok(v) => Some((key_val.key, v)),
                        Err(_) => None,
                    },
                    Err(_) => None,
                }
            }
        }
    }

    /// Try to read a variable-length value at the given key from storage.
    pub fn read<K: AsRef<str>, T: BorshDeserialize>(key: K) -> Option<T> {
        let key = key.as_ref();
        let size = size_of::<T>();
        let result = Vec::with_capacity(size);
        let size = unsafe {
            anoma_tx_read(
                key.as_ptr() as _,
                key.len() as _,
                result.as_ptr() as _,
            )
        };
        if HostEnvResult::is_fail(size) {
            None
        } else {
            let slice =
                unsafe { slice::from_raw_parts(result.as_ptr(), size as _) };
            T::try_from_slice(slice).ok()
        }
    }

    /// Check if the given key is present in storage.
    pub fn has_key(key: impl AsRef<str>) -> bool {
        let key = key.as_ref();
        let found =
            unsafe { anoma_tx_has_key(key.as_ptr() as _, key.len() as _) };
        HostEnvResult::is_success(found)
    }

    /// Write a value at the given key to storage.
    pub fn write<K: AsRef<str>, T: BorshSerialize>(key: K, val: T) {
        let key = key.as_ref();
        let mut buf: Vec<u8> = Vec::with_capacity(size_of::<T>());
        val.serialize(&mut buf).unwrap();
        unsafe {
            anoma_tx_write(
                key.as_ptr() as _,
                key.len() as _,
                buf.as_ptr() as _,
                buf.len() as _,
            )
        };
    }

    /// Delete a value at the given key from storage.
    pub fn delete<K: AsRef<str>, T: BorshSerialize>(key: K) {
        let key = key.as_ref();
        unsafe { anoma_tx_delete(key.as_ptr() as _, key.len() as _) };
    }

    /// Get an iterator with the given prefix
    pub fn iter_prefix<K: AsRef<str>, T: BorshDeserialize>(
        prefix: K,
    ) -> KeyValIterator<T> {
        let prefix = prefix.as_ref();
        let iter_id = unsafe {
            anoma_tx_iter_prefix(prefix.as_ptr() as _, prefix.len() as _)
        };
        KeyValIterator(iter_id, PhantomData)
    }

    /// Insert a verifier
    pub fn insert_verifier(addr: Address) {
        let addr = addr.encode();
        unsafe { anoma_tx_insert_verifier(addr.as_ptr() as _, addr.len() as _) }
    }

    /// Update a validity predicate
    pub fn update_validity_predicate(addr: Address, code: impl AsRef<[u8]>) {
        let addr = addr.encode();
        let code = code.as_ref();
        unsafe {
            anoma_tx_update_validity_predicate(
                addr.as_ptr() as _,
                addr.len() as _,
                code.as_ptr() as _,
                code.len() as _,
            )
        };
    }

    // Initialize a new account
    pub fn init_account(code: impl AsRef<[u8]>) -> Address {
        let code = code.as_ref();
        let result = Vec::with_capacity(0);
        let result_len = unsafe {
            anoma_tx_init_account(
                code.as_ptr() as _,
                code.len() as _,
                result.as_ptr() as _,
            )
        };
        let slice =
            unsafe { slice::from_raw_parts(result.as_ptr(), result_len as _) };
        Address::try_from_slice(slice)
            .expect("Decoding address created by the ledger shouldn't fail")
    }

    /// Get the chain ID
    pub fn get_chain_id() -> String {
        let result = Vec::with_capacity(CHAIN_ID_LENGTH);
        unsafe {
            anoma_tx_get_chain_id(result.as_ptr() as _);
        }
        let slice = unsafe {
            slice::from_raw_parts(result.as_ptr(), CHAIN_ID_LENGTH as _)
        };
        String::from_utf8(slice.to_vec()).expect("Cannot convert the ID string")
    }

    /// Get the committed block height
    pub fn get_block_height() -> BlockHeight {
        BlockHeight(unsafe { anoma_tx_get_block_height() })
    }

    /// Get a block hash
    pub fn get_block_hash() -> BlockHash {
        let result = Vec::with_capacity(BLOCK_HASH_LENGTH);
        unsafe {
            anoma_tx_get_block_hash(result.as_ptr() as _);
        }
        let slice = unsafe {
            slice::from_raw_parts(result.as_ptr(), BLOCK_HASH_LENGTH as _)
        };
        BlockHash::try_from(slice).expect("Cannot convert the hash")
    }

    /// Log a string. The message will be printed at the `tracing::Level::Info`.
    pub fn log_string<T: AsRef<str>>(msg: T) {
        let msg = msg.as_ref();
        unsafe {
            anoma_tx_log_string(msg.as_ptr() as _, msg.len() as _);
        }
    }

    /// These host functions are implemented in the Anoma's [`host_env`]
    /// module. The environment provides calls to them via this C interface.
    extern "C" {
        // Read variable-length data when we don't know the size up-front,
        // returns the size of the value (can be 0), or -1 if the key is
        // not present.
        fn anoma_tx_read(key_ptr: u64, key_len: u64, result_ptr: u64) -> i64;

        // Returns 1 if the key is present, -1 otherwise.
        fn anoma_tx_has_key(key_ptr: u64, key_len: u64) -> i64;

        // Write key/value
        fn anoma_tx_write(
            key_ptr: u64,
            key_len: u64,
            val_ptr: u64,
            val_len: u64,
        );

        // Delete the given key and its value
        fn anoma_tx_delete(key_ptr: u64, key_len: u64);

        // Get an ID of a data iterator with key prefix
        fn anoma_tx_iter_prefix(prefix_ptr: u64, prefix_len: u64) -> u64;

        // Read variable-length data when we don't know the size up-front,
        // returns the size of the value (can be 0), or -1 if the key is not
        // present.
        fn anoma_tx_iter_next(iter_id: u64, result_ptr: u64) -> i64;

        // Insert a verifier
        fn anoma_tx_insert_verifier(addr_ptr: u64, addr_len: u64);

        // Update a validity predicate
        fn anoma_tx_update_validity_predicate(
            addr_ptr: u64,
            addr_len: u64,
            code_ptr: u64,
            code_len: u64,
        );

        // Initialize a new account
        fn anoma_tx_init_account(
            code_ptr: u64,
            code_len: u64,
            result_ptr: u64,
        ) -> u64;

        // Get the chain ID
        fn anoma_tx_get_chain_id(result_ptr: u64);

        // Get the current block height
        fn anoma_tx_get_block_height() -> u64;

        // Get the current block hash
        fn anoma_tx_get_block_hash(result_ptr: u64);

        // Requires a node running with "Info" log level
        fn anoma_tx_log_string(str_ptr: u64, str_len: u64);
    }
}

/// Validity predicate environment imports
pub mod vp {
    pub use core::slice;
    use std::convert::TryFrom;
    use std::marker::PhantomData;
    pub use std::mem::size_of;

    use anoma_shared::types::internal::HostEnvResult;
    use anoma_shared::types::key::ed25519::{PublicKey, Signature};
    use anoma_shared::types::{
        BlockHash, BlockHeight, BLOCK_HASH_LENGTH, CHAIN_ID_LENGTH,
    };
    use anoma_shared::vm::types::KeyVal;
    pub use borsh::{BorshDeserialize, BorshSerialize};

    pub struct PreKeyValIterator<T>(pub u64, pub PhantomData<T>);
    pub struct PostKeyValIterator<T>(pub u64, pub PhantomData<T>);

    /// Try to read a variable-length value at the given key from storage before
    /// transaction execution.
    pub fn read_pre<K: AsRef<str>, T: BorshDeserialize>(key: K) -> Option<T> {
        let key = key.as_ref();
        let size = size_of::<T>();
        let result = Vec::with_capacity(size);
        let size = unsafe {
            anoma_vp_read_pre(
                key.as_ptr() as _,
                key.len() as _,
                result.as_ptr() as _,
            )
        };
        if HostEnvResult::is_fail(size) {
            None
        } else {
            let slice =
                unsafe { slice::from_raw_parts(result.as_ptr(), size as _) };
            T::try_from_slice(slice).ok()
        }
    }

    /// Try to read a variable-length value at the given key from storage after
    /// transaction execution.
    pub fn read_post<K: AsRef<str>, T: BorshDeserialize>(key: K) -> Option<T> {
        let key = key.as_ref();
        let size = size_of::<T>();
        let result = Vec::with_capacity(size);
        let size = unsafe {
            anoma_vp_read_post(
                key.as_ptr() as _,
                key.len() as _,
                result.as_ptr() as _,
            )
        };
        if HostEnvResult::is_fail(size) {
            None
        } else {
            let slice =
                unsafe { slice::from_raw_parts(result.as_ptr(), size as _) };
            T::try_from_slice(slice).ok()
        }
    }

    /// Check if the given key was present in storage before transaction
    /// execution.
    pub fn has_key_pre(key: impl AsRef<str>) -> bool {
        let key = key.as_ref();
        let found =
            unsafe { anoma_vp_has_key_pre(key.as_ptr() as _, key.len() as _) };
        HostEnvResult::is_success(found)
    }

    /// Check if the given key is present in storage after transaction
    /// execution.
    pub fn has_key_post(key: impl AsRef<str>) -> bool {
        let key = key.as_ref();
        let found =
            unsafe { anoma_vp_has_key_post(key.as_ptr() as _, key.len() as _) };
        HostEnvResult::is_success(found)
    }

    /// Get an iterator with the given prefix before transaction execution
    pub fn iter_prefix_pre<K: AsRef<str>, T: BorshDeserialize>(
        prefix: K,
    ) -> PreKeyValIterator<T> {
        let prefix = prefix.as_ref();
        let iter_id = unsafe {
            anoma_vp_iter_prefix(prefix.as_ptr() as _, prefix.len() as _)
        };
        PreKeyValIterator(iter_id, PhantomData)
    }

    impl<T: BorshDeserialize> Iterator for PreKeyValIterator<T> {
        type Item = (String, T);

        fn next(&mut self) -> Option<(String, T)> {
            let result: Vec<u8> = Vec::with_capacity(0);
            let size =
                unsafe { anoma_vp_iter_pre_next(self.0, result.as_ptr() as _) };
            if HostEnvResult::is_fail(size) {
                None
            } else {
                let slice = unsafe {
                    slice::from_raw_parts(result.as_ptr(), size as _)
                };
                match KeyVal::try_from_slice(slice) {
                    Ok(key_val) => match T::try_from_slice(&key_val.val) {
                        Ok(v) => Some((key_val.key, v)),
                        Err(_) => None,
                    },
                    Err(_) => None,
                }
            }
        }
    }

    /// Get an iterator with the given prefix after transaction execution
    pub fn iter_prefix_post<K: AsRef<str>, T: BorshDeserialize>(
        prefix: K,
    ) -> PostKeyValIterator<T> {
        let prefix = prefix.as_ref();
        let iter_id = unsafe {
            anoma_vp_iter_prefix(prefix.as_ptr() as _, prefix.len() as _)
        };
        PostKeyValIterator(iter_id, PhantomData)
    }

    impl<T: BorshDeserialize> Iterator for PostKeyValIterator<T> {
        type Item = (String, T);

        fn next(&mut self) -> Option<(String, T)> {
            let result: Vec<u8> = Vec::with_capacity(0);
            let size = unsafe {
                anoma_vp_iter_post_next(self.0, result.as_ptr() as _)
            };
            if HostEnvResult::is_fail(size) {
                None
            } else {
                let slice = unsafe {
                    slice::from_raw_parts(result.as_ptr(), size as _)
                };
                match KeyVal::try_from_slice(slice) {
                    Ok(key_val) => match T::try_from_slice(&key_val.val) {
                        Ok(v) => Some((key_val.key, v)),
                        Err(_) => None,
                    },
                    Err(_) => None,
                }
            }
        }
    }

    /// Get the chain ID
    pub fn get_chain_id() -> String {
        let result = Vec::with_capacity(CHAIN_ID_LENGTH);
        unsafe {
            anoma_vp_get_chain_id(result.as_ptr() as _);
        }
        let slice = unsafe {
            slice::from_raw_parts(result.as_ptr(), CHAIN_ID_LENGTH as _)
        };
        String::from_utf8(slice.to_vec()).expect("Cannot convert the ID string")
    }

    /// Get the committed block height
    pub fn get_block_height() -> BlockHeight {
        BlockHeight(unsafe { anoma_vp_get_block_height() })
    }

    /// Get a block hash
    pub fn get_block_hash() -> BlockHash {
        let result = Vec::with_capacity(BLOCK_HASH_LENGTH);
        unsafe {
            anoma_vp_get_block_hash(result.as_ptr() as _);
        }
        let slice = unsafe {
            slice::from_raw_parts(result.as_ptr(), BLOCK_HASH_LENGTH as _)
        };
        BlockHash::try_from(slice).expect("Cannot convert the hash")
    }

    /// Verify a transaction signature. The signature is expected to have been
    /// produced on the data concatenated with the transaction code.
    pub fn verify_tx_signature(
        pk: &PublicKey,
        data: &[u8],
        sig: &Signature,
    ) -> bool {
        let pk = BorshSerialize::try_to_vec(pk).unwrap();
        let sig = BorshSerialize::try_to_vec(sig).unwrap();
        let valid = unsafe {
            anoma_vp_verify_tx_signature(
                pk.as_ptr() as _,
                pk.len() as _,
                data.as_ptr() as _,
                data.len() as _,
                sig.as_ptr() as _,
                sig.len() as _,
            )
        };
        HostEnvResult::is_success(valid)
    }

    /// Log a string. The message will be printed at the `tracing::Level::Info`.
    pub fn log_string<T: AsRef<str>>(msg: T) {
        let msg = msg.as_ref();
        unsafe {
            anoma_vp_log_string(msg.as_ptr() as _, msg.len() as _);
        }
    }

    /// Evaluate a validity predicate with given data. The address, changed
    /// storage keys and verifiers will have the same values as the input to
    /// caller's validity predicate.
    pub fn eval(vp_code: Vec<u8>, input_data: Vec<u8>) -> bool {
        let result = unsafe {
            anoma_vp_eval(
                vp_code.as_ptr() as _,
                vp_code.len() as _,
                input_data.as_ptr() as _,
                input_data.len() as _,
            )
        };
        HostEnvResult::is_success(result)
    }

    /// These host functions are implemented in the Anoma's [`host_env`]
    /// module. The environment provides calls to them via this C interface.
    extern "C" {
        // Read variable-length prior state when we don't know the size
        // up-front, returns the size of the value (can be 0), or -1 if
        // the key is not present.
        fn anoma_vp_read_pre(
            key_ptr: u64,
            key_len: u64,
            result_ptr: u64,
        ) -> i64;

        // Read variable-length posterior state when we don't know the size
        // up-front, returns the size of the value (can be 0), or -1 if
        // the key is not present.
        fn anoma_vp_read_post(
            key_ptr: u64,
            key_len: u64,
            result_ptr: u64,
        ) -> i64;

        // Returns 1 if the key is present in prior state, -1 otherwise.
        fn anoma_vp_has_key_pre(key_ptr: u64, key_len: u64) -> i64;

        // Returns 1 if the key is present in posterior state, -1 otherwise.
        fn anoma_vp_has_key_post(key_ptr: u64, key_len: u64) -> i64;

        // Get an ID of a data iterator with key prefix
        fn anoma_vp_iter_prefix(prefix_ptr: u64, prefix_len: u64) -> u64;

        // Read variable-length prior state when we don't know the size
        // up-front, returns the size of the value (can be 0), or -1 if
        // the key is not present.
        fn anoma_vp_iter_pre_next(iter_id: u64, result_ptr: u64) -> i64;

        // Read variable-length posterior state when we don't know the size
        // up-front, returns the size of the value (can be 0), or -1 if the
        // key is not present.
        fn anoma_vp_iter_post_next(iter_id: u64, result_ptr: u64) -> i64;

        // Get the chain ID
        fn anoma_vp_get_chain_id(result_ptr: u64);

        // Get the current block height
        fn anoma_vp_get_block_height() -> u64;

        // Get the current block hash
        fn anoma_vp_get_block_hash(result_ptr: u64);

        // Verify a transaction signature
        fn anoma_vp_verify_tx_signature(
            pk_ptr: u64,
            pk_len: u64,
            data_ptr: u64,
            data_len: u64,
            sig_ptr: u64,
            sig_len: u64,
        ) -> i64;

        // Requires a node running with "Info" log level
        fn anoma_vp_log_string(str_ptr: u64, str_len: u64);

        fn anoma_vp_eval(
            vp_code_ptr: u64,
            vp_code_len: u64,
            input_data_ptr: u64,
            input_data_len: u64,
        ) -> i64;
    }
}

/// Matchmaker environment imports
pub mod matchmaker {
    pub use core::slice;
    use std::collections::HashSet;

    pub use borsh::{BorshDeserialize, BorshSerialize};

    /// Send a transaction with the `tx_data` and the `tx_code` to the ledger
    /// given in matchmaker parameters (`--tx-code-path` and
    /// `--ledger-address`).
    pub fn send_match(tx_data: Vec<u8>) {
        unsafe {
            anoma_mm_send_match(tx_data.as_ptr() as _, tx_data.len() as _)
        };
    }

    /// Update the matchmaker state. This state will be pass on the next run of
    /// the matchmaker.
    pub fn update_data(data: Vec<u8>) {
        unsafe { anoma_mm_update_data(data.as_ptr() as _, data.len() as _) };
    }

    /// Remove the intents from the matchmaker intent mempool, to call when they
    /// are fulfilled or outdated.
    pub fn remove_intents(intents_id: HashSet<Vec<u8>>) {
        let intents_id_bytes = intents_id.try_to_vec().unwrap();
        unsafe {
            anoma_mm_remove_intents(
                intents_id_bytes.as_ptr() as _,
                intents_id_bytes.len() as _,
            )
        };
    }

    /// Log a string. The message will be printed at the `tracing::Level::Info`.
    pub fn log_string<T: AsRef<str>>(msg: T) {
        let msg = msg.as_ref();
        unsafe {
            anoma_mm_log_string(msg.as_ptr() as _, msg.len() as _);
        }
    }

    /// These host functions are implemented in the Anoma's [`host_env`]
    /// module. The environment provides calls to them via this C interface.
    extern "C" {
        // Inject a transaction from matchmaker's matched intents to the ledger
        fn anoma_mm_send_match(data_ptr: u64, data_len: u64);

        fn anoma_mm_update_data(data_ptr: u64, data_len: u64);

        fn anoma_mm_remove_intents(intents_id_ptr: u64, intents_id_len: u64);

        // Requires a node running with "Info" log level
        fn anoma_mm_log_string(str_ptr: u64, str_len: u64);
    }
}

/// Filter environment imports
pub mod filter {
    pub use core::slice;

    pub use borsh::{BorshDeserialize, BorshSerialize};

    /// Log a string. The message will be printed at the `tracing::Level::Info`.
    pub fn log_string<T: AsRef<str>>(msg: T) {
        let msg = msg.as_ref();
        unsafe {
            anoma_filter_log_string(msg.as_ptr() as _, msg.len() as _);
        }
    }

    /// These host functions are implemented in the Anoma's [`host_env`]
    /// module. The environment provides calls to them via this C interface.
    extern "C" {
        // Requires a node running with "Info" log level
        fn anoma_filter_log_string(str_ptr: u64, str_len: u64);
    }
}
