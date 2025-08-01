use std::collections::HashSet;

#[derive(Debug)]
pub enum MatchmakerMessage {
    InjectTx(Vec<u8>),
    RemoveIntents(HashSet<Vec<u8>>),
    UpdateData(Vec<u8>),
}
