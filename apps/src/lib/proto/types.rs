use std::collections::hash_map::DefaultHasher;
use std::convert::{TryFrom, TryInto};
use std::hash::{Hash, Hasher};

use prost::Message;
use prost_types::Timestamp;
use thiserror::Error;

use super::generated::{services, types};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error decoding a transaction from bytes: {0}")]
    TxDecodingError(prost::DecodeError),
    #[error("Error decoding an IntentGossipMessage from bytes: {0}")]
    IntentDecodingError(prost::DecodeError),
    #[error("Error decoding an DkgGossipMessage from bytes: {0}")]
    DkgDecodingError(prost::DecodeError),
    #[error("Intent is empty")]
    NoIntentError,
    #[error("Dkg is empty")]
    NoDkgError,
    #[error("Timestamp is empty")]
    NoTimestampError,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub struct Tx {
    pub code: Vec<u8>,
    pub data: Option<Vec<u8>>,
    pub timestamp: Timestamp,
}

impl TryFrom<&[u8]> for Tx {
    type Error = Error;

    fn try_from(tx_bytes: &[u8]) -> Result<Self> {
        let tx = types::Tx::decode(tx_bytes).map_err(Error::TxDecodingError)?;
        let timestamp = match tx.timestamp {
            Some(t) => t,
            None => return Err(Error::NoTimestampError),
        };
        Ok(Tx {
            code: tx.code,
            data: tx.data,
            timestamp,
        })
    }
}

impl From<Tx> for types::Tx {
    fn from(tx: Tx) -> Self {
        types::Tx {
            code: tx.code.clone(),
            data: tx.data.clone(),
            timestamp: Some(tx.timestamp),
        }
    }
}

impl Tx {
    pub fn new(code: Vec<u8>, data: Option<Vec<u8>>) -> Self {
        Tx {
            code,
            data,
            timestamp: std::time::SystemTime::now().into(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        let tx: types::Tx = self.clone().into();
        tx.encode(&mut bytes)
            .expect("encoding a transaction failed");
        bytes
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntentGossipMessage {
    pub intent: Intent,
}

impl TryFrom<&[u8]> for IntentGossipMessage {
    type Error = Error;

    fn try_from(intent_bytes: &[u8]) -> Result<Self> {
        let intent = types::IntentGossipMessage::decode(intent_bytes)
            .map_err(Error::IntentDecodingError)?;
        match &intent.msg {
            Some(types::intent_gossip_message::Msg::Intent(intent)) => {
                Ok(IntentGossipMessage {
                    intent: intent.clone().try_into()?,
                })
            }
            None => Err(Error::NoIntentError),
        }
    }
}

impl From<IntentGossipMessage> for types::IntentGossipMessage {
    fn from(message: IntentGossipMessage) -> Self {
        types::IntentGossipMessage {
            msg: Some(types::intent_gossip_message::Msg::Intent(
                message.intent.into(),
            )),
        }
    }
}

impl IntentGossipMessage {
    pub fn new(intent: Intent) -> Self {
        IntentGossipMessage { intent }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        let message: types::IntentGossipMessage = self.clone().into();
        message
            .encode(&mut bytes)
            .expect("encoding an intent gossip message failed");
        bytes
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct DkgGossipMessage {
    pub dkg: Dkg,
}

impl TryFrom<&[u8]> for DkgGossipMessage {
    type Error = Error;

    fn try_from(dkg_bytes: &[u8]) -> Result<Self> {
        let message = types::DkgGossipMessage::decode(dkg_bytes)
            .map_err(Error::DkgDecodingError)?;
        match &message.dkg_message {
            Some(types::dkg_gossip_message::DkgMessage::Dkg(dkg)) => {
                Ok(DkgGossipMessage {
                    dkg: dkg.clone().into(),
                })
            }
            None => Err(Error::NoDkgError),
        }
    }
}

impl From<DkgGossipMessage> for types::DkgGossipMessage {
    fn from(message: DkgGossipMessage) -> Self {
        types::DkgGossipMessage {
            dkg_message: Some(types::dkg_gossip_message::DkgMessage::Dkg(
                message.dkg.into(),
            )),
        }
    }
}

#[allow(dead_code)]
impl DkgGossipMessage {
    pub fn new(dkg: Dkg) -> Self {
        DkgGossipMessage { dkg }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        let message: types::DkgGossipMessage = self.clone().into();
        message
            .encode(&mut bytes)
            .expect("encoding a DKG gossip message failed");
        bytes
    }
}

pub enum RpcMessage {
    IntentMessage(IntentMessage),
    SubscribeTopicMessage(SubscribeTopicMessage),
    Dkg(Dkg),
}

impl From<RpcMessage> for services::RpcMessage {
    fn from(message: RpcMessage) -> Self {
        let message = match message {
            RpcMessage::IntentMessage(m) => {
                services::rpc_message::Message::Intent(m.into())
            }
            RpcMessage::SubscribeTopicMessage(m) => {
                services::rpc_message::Message::Topic(m.into())
            }
            RpcMessage::Dkg(d) => services::rpc_message::Message::Dkg(d.into()),
        };
        services::RpcMessage {
            message: Some(message),
        }
    }
}

impl RpcMessage {
    pub fn new_intent(intent: Intent, topic: String) -> Self {
        RpcMessage::IntentMessage(IntentMessage::new(intent, topic))
    }

    pub fn new_topic(topic: String) -> Self {
        RpcMessage::SubscribeTopicMessage(SubscribeTopicMessage::new(topic))
    }

    pub fn new_dkg(dkg: Dkg) -> Self {
        RpcMessage::Dkg(dkg)
    }
}

#[derive(Debug, PartialEq)]
pub struct IntentMessage {
    pub intent: Intent,
    pub topic: String,
}

impl TryFrom<services::IntentMessage> for IntentMessage {
    type Error = Error;

    fn try_from(message: services::IntentMessage) -> Result<Self> {
        match message.intent {
            Some(intent) => Ok(IntentMessage {
                intent: intent.try_into()?,
                topic: message.topic,
            }),
            None => Err(Error::NoIntentError),
        }
    }
}

impl From<IntentMessage> for services::IntentMessage {
    fn from(message: IntentMessage) -> Self {
        services::IntentMessage {
            intent: Some(message.intent.into()),
            topic: message.topic,
        }
    }
}

impl IntentMessage {
    pub fn new(intent: Intent, topic: String) -> Self {
        IntentMessage { intent, topic }
    }
}

#[derive(Debug, PartialEq)]
pub struct SubscribeTopicMessage {
    pub topic: String,
}

impl From<services::SubscribeTopicMessage> for SubscribeTopicMessage {
    fn from(message: services::SubscribeTopicMessage) -> Self {
        SubscribeTopicMessage {
            topic: message.topic,
        }
    }
}

impl From<SubscribeTopicMessage> for services::SubscribeTopicMessage {
    fn from(message: SubscribeTopicMessage) -> Self {
        services::SubscribeTopicMessage {
            topic: message.topic,
        }
    }
}

impl SubscribeTopicMessage {
    pub fn new(topic: String) -> Self {
        SubscribeTopicMessage { topic }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Intent {
    pub data: Vec<u8>,
    pub timestamp: Timestamp,
}

impl TryFrom<types::Intent> for Intent {
    type Error = Error;

    fn try_from(intent: types::Intent) -> Result<Self> {
        let timestamp = match intent.timestamp {
            Some(t) => t,
            None => return Err(Error::NoTimestampError),
        };
        Ok(Intent {
            data: intent.data,
            timestamp,
        })
    }
}

impl From<Intent> for types::Intent {
    fn from(intent: Intent) -> Self {
        types::Intent {
            data: intent.data,
            timestamp: Some(intent.timestamp),
        }
    }
}

impl Intent {
    pub fn new(data: Vec<u8>) -> Self {
        Intent {
            data,
            timestamp: std::time::SystemTime::now().into(),
        }
    }

    pub fn id(&self) -> IntentId {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        IntentId::from(hasher.finish().to_string())
    }
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for Intent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
        let timestamp: std::time::SystemTime = self.timestamp.clone().into();
        timestamp.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntentId(pub Vec<u8>);

impl<T: Into<Vec<u8>>> From<T> for IntentId {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct Dkg {
    pub data: String,
}

impl From<types::Dkg> for Dkg {
    fn from(dkg: types::Dkg) -> Self {
        Dkg { data: dkg.data }
    }
}

impl From<Dkg> for types::Dkg {
    fn from(dkg: Dkg) -> Self {
        types::Dkg { data: dkg.data }
    }
}

#[allow(dead_code)]
impl Dkg {
    pub fn new(data: String) -> Self {
        Dkg { data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx() {
        let code = "wasm code".as_bytes().to_owned();
        let data = Some("arbitrary data".as_bytes().to_owned());
        let tx = Tx::new(code.clone(), data.clone());

        let bytes = tx.to_bytes();
        let tx_from_bytes =
            Tx::try_from(bytes.as_ref()).expect("decoding failed");
        assert_eq!(tx_from_bytes, tx);

        let types_tx = types::Tx {
            code,
            data,
            timestamp: None,
        };
        let mut bytes = vec![];
        types_tx.encode(&mut bytes).expect("encoding failed");
        match Tx::try_from(bytes.as_ref()) {
            Err(Error::NoTimestampError) => {}
            _ => panic!("unexpected result"),
        }
    }

    #[test]
    fn test_intent_gossip_message() {
        let data = "arbitrary data".as_bytes().to_owned();
        let intent = Intent::new(data);
        let message = IntentGossipMessage::new(intent.clone());

        let bytes = message.to_bytes();
        let message_from_bytes = IntentGossipMessage::try_from(bytes.as_ref())
            .expect("decoding failed");
        assert_eq!(message_from_bytes, message);
    }

    #[test]
    fn test_dkg_gossip_message() {
        let data = "arbitrary string".to_owned();
        let dkg = Dkg::new(data);
        let message = DkgGossipMessage::new(dkg.clone());

        let bytes = message.to_bytes();
        let message_from_bytes = DkgGossipMessage::try_from(bytes.as_ref())
            .expect("decoding failed");
        assert_eq!(message_from_bytes, message);
    }

    #[test]
    fn test_intent_message() {
        let data = "arbitrary data".as_bytes().to_owned();
        let intent = Intent::new(data);
        let topic = "arbitrary string".to_owned();
        let intent_message = IntentMessage::new(intent.clone(), topic.clone());

        let intent_rpc_message = RpcMessage::new_intent(intent, topic);
        let services_rpc_message: services::RpcMessage =
            intent_rpc_message.into();
        match services_rpc_message.message {
            Some(services::rpc_message::Message::Intent(i)) => {
                let message_from_types =
                    IntentMessage::try_from(i).expect("no intent");
                assert_eq!(intent_message, message_from_types);
            }
            _ => panic!("no intent message"),
        }
    }

    #[test]
    fn test_topic_message() {
        let topic = "arbitrary string".to_owned();
        let topic_message = SubscribeTopicMessage::new(topic.clone());

        let topic_rpc_message = RpcMessage::new_topic(topic.clone());
        let services_rpc_message: services::RpcMessage =
            topic_rpc_message.into();
        match services_rpc_message.message {
            Some(services::rpc_message::Message::Topic(t)) => {
                let message_from_types = SubscribeTopicMessage::from(t);
                assert_eq!(topic_message, message_from_types);
            }
            _ => panic!("no intent message"),
        }
    }

    #[test]
    fn test_intent() {
        let data = "arbitrary data".as_bytes().to_owned();
        let intent = Intent::new(data.clone());

        let types_intent: types::Intent = intent.clone().into();
        let intent_from_types =
            Intent::try_from(types_intent).expect("no timestamp");
        assert_eq!(intent_from_types, intent);

        let types_intent = types::Intent {
            data,
            timestamp: None,
        };
        match Intent::try_from(types_intent) {
            Err(Error::NoTimestampError) => {}
            _ => panic!("unexpected result"),
        }
    }

    #[test]
    fn test_dkg() {
        let data = "arbitrary string".to_owned();
        let dkg = Dkg::new(data.clone());

        let types_dkg: types::Dkg = dkg.clone().into();
        let dkg_from_types = Dkg::from(types_dkg);
        assert_eq!(dkg_from_types, dkg);
    }
}
