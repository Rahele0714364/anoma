use std::collections::HashSet;

use anoma_shared::ledger::gas::BlockGasMeter;
use anoma_shared::ledger::storage::mockdb::MockDB;
use anoma_shared::ledger::storage::testing::TestStorage;
use anoma_shared::ledger::storage::write_log::WriteLog;
use anoma_shared::types::{Address, Key};
use anoma_shared::vm;
use anoma_shared::vm::prefix_iter::PrefixIterators;

/// This module combines the native host function implementations from
/// `native_tx_host_env` with the functions exposed to the tx wasm
/// that will call to the native functions, instead of interfacing via a
/// wasm runtime. It can be used for host environment integration tests.
pub mod tx_host_env {
    pub use anoma_vm_env::imports::tx::*;

    pub use super::native_tx_host_env::*;
}

/// Host environment structures required for transactions.
pub struct TestTxEnv {
    pub storage: TestStorage,
    pub write_log: WriteLog,
    pub iterators: PrefixIterators<'static, MockDB>,
    pub verifiers: HashSet<Address>,
    pub gas_meter: BlockGasMeter,
}

impl Default for TestTxEnv {
    fn default() -> Self {
        Self {
            storage: TestStorage::default(),
            write_log: WriteLog::default(),
            iterators: PrefixIterators::default(),
            verifiers: HashSet::default(),
            gas_meter: BlockGasMeter::default(),
        }
    }
}

impl TestTxEnv {
    pub fn all_touched_storage_keys(&self) -> Vec<Key> {
        self.write_log.get_keys()
    }
}

/// Initialize the host environment inside the [`tx_host_env`] module.
#[allow(dead_code)]
pub fn init_tx_env(
    TestTxEnv {
        storage,
        write_log,
        iterators,
        verifiers,
        gas_meter,
    }: &mut TestTxEnv,
) {
    tx_host_env::ENV.with(|env| {
        *env.borrow_mut() = Some({
            vm::host_env::testing::tx_env(
                storage, write_log, iterators, verifiers, gas_meter,
            )
        })
    });
}

/// This module allows to test code with tx host environment functions.
/// It keeps a thread-local global `TxEnv`, which is passed to any of
/// invoked host environment functions and so it must be initialized
/// before the test.
mod native_tx_host_env {

    use std::cell::RefCell;

    use anoma_shared::ledger::storage::testing::Sha256Hasher;
    use anoma_shared::vm::host_env::*;
    use anoma_shared::vm::memory::testing::NativeMemory;
    // TODO replace with `std::concat_idents` once stabilized (https://github.com/rust-lang/rust/issues/29599)
    use concat_idents::concat_idents;

    use super::*;

    thread_local! {
        pub static ENV: RefCell<Option<TxEnv<'static, NativeMemory, MockDB, Sha256Hasher>>> = RefCell::new(None);
    }

    /// A helper macro to create implementations of the host environment
    /// functions exported to wasm, which uses the environment from the
    /// `ENV` variable.
    macro_rules! native_host_fn {
            // unit return type
            ( $fn:ident ( $($arg:ident : $type:ty),* $(,)?) ) => {
                concat_idents!(extern_fn_name = anoma, _, $fn {
                    #[no_mangle]
                    extern "C" fn extern_fn_name( $($arg: $type),* ) {
                        ENV.with(|env| {
                            let env = env.borrow_mut();
                            let env = env.as_ref().expect("Did you forget to initialize the ENV?");

                            $fn( &env, $($arg),* )
                        })
                    }
                });
            };

            // non-unit return type
            ( $fn:ident ( $($arg:ident : $type:ty),* $(,)?) -> $ret:ty ) => {
                concat_idents!(extern_fn_name = anoma, _, $fn {
                    #[no_mangle]
                    extern "C" fn extern_fn_name( $($arg: $type),* ) -> $ret {
                        ENV.with(|env| {
                            let env = env.borrow_mut();
                            let env = env.as_ref().expect("Did you forget to initialize the ENV?");

                            $fn( &env, $($arg),* )
                        })
                    }
                });
            }
        }

    // Implement all the exported functions from
    // [`anoma_vm_env::imports::tx`] `extern "C"` section.
    native_host_fn!(tx_read(key_ptr: u64, key_len: u64, result_ptr: u64) -> i64);
    native_host_fn!(tx_has_key(key_ptr: u64, key_len: u64) -> i64);
    native_host_fn!(tx_write(
        key_ptr: u64,
        key_len: u64,
        val_ptr: u64,
        val_len: u64
    ));
    native_host_fn!(tx_delete(key_ptr: u64, key_len: u64));
    native_host_fn!(tx_iter_prefix(prefix_ptr: u64, prefix_len: u64) -> u64);
    native_host_fn!(tx_iter_next(iter_id: u64, result_ptr: u64) -> i64);
    native_host_fn!(tx_insert_verifier(addr_ptr: u64, addr_len: u64));
    native_host_fn!(tx_update_validity_predicate(
        addr_ptr: u64,
        addr_len: u64,
        code_ptr: u64,
        code_len: u64,
    ));
    native_host_fn!(tx_init_account(code_ptr: u64, code_len: u64, result_ptr: u64) -> u64);
    native_host_fn!(tx_get_chain_id(result_ptr: u64));
    native_host_fn!(tx_get_block_height() -> u64);
    native_host_fn!(tx_get_block_hash(result_ptr: u64));
    native_host_fn!(tx_log_string(str_ptr: u64, str_len: u64));
}
