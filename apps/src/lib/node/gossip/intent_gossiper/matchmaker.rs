use std::sync::{Arc, Mutex};

use anoma_shared::gossip::mm::MmHost;
use anoma_shared::types::key::ed25519::SignedTxData;
use anoma_shared::vm::wasm::runner::{self, MmRunner};
use borsh::BorshSerialize;
use tendermint::net;
use tendermint_rpc::{Client, HttpClient};
use thiserror::Error;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use super::filter::Filter;
use super::mempool::{self, IntentMempool};
use crate::proto::{Intent, IntentId, Tx};
use crate::types::MatchmakerMessage;
use crate::{config, wallet};

#[derive(Debug)]
pub struct Matchmaker {
    mempool: IntentMempool,
    filter: Option<Filter>,
    matchmaker_code: Vec<u8>,
    tx_code: Vec<u8>,
    // the matchmaker's state as arbitrary bytes
    data: Vec<u8>,
    ledger_address: net::Address,
    // TODO this doesn't have to be a mutex as it's just a Sender which is
    // thread-safe
    wasm_host: Arc<Mutex<WasmHost>>,
}

#[derive(Debug)]
struct WasmHost(Sender<MatchmakerMessage>);

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to add intent to mempool: {0}")]
    MempoolFailed(mempool::Error),
    #[error("Failed to run matchmaker prog: {0}")]
    RunnerFailed(runner::Error),
    #[error("Failed to read file: {0}")]
    FileFailed(std::io::Error),
    #[error("Failed to create filter: {0}")]
    FilterInit(super::filter::Error),
    #[error("Failed to run filter: {0}")]
    Filter(super::filter::Error),
}

type Result<T> = std::result::Result<T, Error>;

impl MmHost for WasmHost {
    fn remove_intents(&self, intents_id: std::collections::HashSet<Vec<u8>>) {
        self.0
            .try_send(MatchmakerMessage::RemoveIntents(intents_id))
            .expect("Sending matchmaker message")
    }

    fn inject_tx(&self, tx_data: Vec<u8>) {
        self.0
            .try_send(MatchmakerMessage::InjectTx(tx_data))
            .expect("Sending matchmaker message")
    }

    fn update_data(&self, data: Vec<u8>) {
        self.0
            .try_send(MatchmakerMessage::UpdateData(data))
            .expect("Sending matchmaker message")
    }
}

impl Matchmaker {
    pub fn new(
        config: &config::Matchmaker,
    ) -> Result<(Self, Receiver<MatchmakerMessage>)> {
        let (inject_mm_message, receiver_mm_message) = channel(100);
        let matchmaker_code =
            std::fs::read(&config.matchmaker).map_err(Error::FileFailed)?;
        let tx_code =
            std::fs::read(&config.tx_code).map_err(Error::FileFailed)?;
        let filter = config
            .filter
            .as_ref()
            .map(Filter::from_file)
            .transpose()
            .map_err(Error::FilterInit)?;

        Ok((
            Self {
                mempool: IntentMempool::new(),
                filter,
                matchmaker_code,
                tx_code,
                data: Vec::new(),
                ledger_address: config.ledger_address.clone(),
                wasm_host: Arc::new(Mutex::new(WasmHost(inject_mm_message))),
            },
            receiver_mm_message,
        ))
    }

    // returns true if no filter is define for that matchmaker
    fn apply_filter(&self, intent: &Intent) -> Result<bool> {
        self.filter
            .as_ref()
            .map(|f| f.validate(intent))
            .transpose()
            .map(|v| v.unwrap_or(true))
            .map_err(Error::Filter)
    }

    // add the intent to the matchmaker mempool and tries to find a match for
    // that intent
    pub fn try_match_intent(&mut self, intent: &Intent) -> Result<bool> {
        if self.apply_filter(intent)? {
            self.mempool
                .put(intent.clone())
                .map_err(Error::MempoolFailed)?;
            let matchmaker_runner = MmRunner::new();
            Ok(matchmaker_runner
                .run(
                    &self.matchmaker_code.clone(),
                    &self.data,
                    &intent.id().0,
                    &intent.data,
                    self.wasm_host.clone(),
                )
                .map_err(Error::RunnerFailed)
                .unwrap())
        } else {
            Ok(false)
        }
    }

    pub async fn handle_mm_message(&mut self, mm_message: MatchmakerMessage) {
        match mm_message {
            MatchmakerMessage::InjectTx(tx_data) => {
                let tx_code = self.tx_code.clone();
                let keypair = wallet::matchmaker_keypair();
                let signed = SignedTxData::new(&keypair, tx_data, &tx_code);
                let signed_bytes = signed
                    .try_to_vec()
                    .expect("Couldn't encode signed matchmaker tx data");
                let tx = Tx {
                    code: tx_code,
                    data: Some(signed_bytes),
                    timestamp: std::time::SystemTime::now().into(),
                };

                let tx_bytes = tx.to_bytes();

                let client =
                    HttpClient::new(self.ledger_address.clone()).unwrap();
                let response =
                    client.broadcast_tx_commit(tx_bytes.into()).await;
                println!("{:#?}", response);
            }
            MatchmakerMessage::RemoveIntents(intents_id) => {
                intents_id.into_iter().for_each(|intent_id| {
                    self.mempool.remove(&IntentId::from(intent_id));
                });
            }
            MatchmakerMessage::UpdateData(mm_data) => {
                self.data = mm_data;
            }
        }
    }
}
