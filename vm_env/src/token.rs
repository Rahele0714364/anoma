use std::collections::HashSet;

use anoma_shared::types::token::{self, Amount, Change};
use anoma_shared::types::{Address, Key};

/// A token validity predicate.
pub fn vp(
    token: &Address,
    keys_changed: &[Key],
    verifiers: &HashSet<Address>,
) -> bool {
    use crate::imports::vp;

    let mut change: Change = 0;
    let all_checked = keys_changed.iter().all(|key| {
        match token::is_balance_key(token, key) {
            None => {
                // deny any other keys
                false
            }
            Some(owner) => {
                // accumulate the change
                let key = key.to_string();
                let pre: Amount = vp::read_pre(&key).unwrap_or_default();
                let post: Amount = vp::read_post(&key).unwrap_or_default();
                let this_change = post.change() - pre.change();
                change += this_change;
                // make sure that the spender approved the transaction
                if this_change < 0 {
                    return verifiers.contains(owner);
                }
                true
            }
        }
    });
    all_checked && change == 0
}

/// A token transfer that can be used in a transaction.
pub fn transfer(
    src: &Address,
    dest: &Address,
    token: &Address,
    amount: Amount,
) {
    use crate::imports::tx;

    let src_key = token::balance_key(token, src);
    let dest_key = token::balance_key(token, dest);
    let src_bal: Option<Amount> = tx::read(&src_key.to_string());
    match src_bal {
        None => {
            tx::log_string(format!("src {} has no balance", src));
            unreachable!()
        }
        Some(mut src_bal) => {
            src_bal.spend(&amount);
            let mut dest_bal: Amount =
                tx::read(&dest_key.to_string()).unwrap_or_default();
            dest_bal.receive(&amount);
            tx::write(&src_key.to_string(), src_bal);
            tx::write(&dest_key.to_string(), dest_bal);
        }
    }
}
