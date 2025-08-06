use borsh::{BorshDeserialize, BorshSerialize};

/// Used to distinguish message types that are sign by account keys, to avoid an
/// abuse of signed messages as something else.
///
/// This prefix must be at the first four bytes of a message body that is
/// signed under this signature scheme.
///
/// The scheme is a draft introduced to avoid security issues with the
/// implementation of meta transactions (NEP-366) but will eventually be
/// standardized with NEP-461 that solves the problem more generally.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    BorshDeserialize,
    BorshSerialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct MessageDiscriminant {
    /// The unique prefix, serialized in little-endian by borsh.
    discriminant: u32,
}

const MIN_ON_CHAIN_DISCRIMINANT: u32 = 1 << 30;
const NEP_366_META_TRANSACTIONS: u32 = MIN_ON_CHAIN_DISCRIMINANT + 366;

/// A wrapper around a message that should be signed using this scheme.
///
/// Only used for constructing a signature, not used to transmit messages. The
/// discriminant prefix is implicit and should be known by the receiver based on
/// the context in which the message is received.
#[derive(BorshSerialize)]
pub struct SignableMessage<'a, T> {
    pub discriminant: MessageDiscriminant,
    pub msg: &'a T,
}

impl<'a, T: BorshSerialize> SignableMessage<'a, T> {
    pub fn new(msg: &'a T, ty: SignableMessageType) -> Self {
        let discriminant = ty.into();
        Self { discriminant, msg }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum SignableMessageType {
    /// A delegate action, intended for a relayer to included it in an action list of a transaction.
    DelegateAction,
}

impl From<SignableMessageType> for MessageDiscriminant {
    fn from(ty: SignableMessageType) -> Self {
        // unwrapping here is ok, we know the constant NEP numbers used are in range
        match ty {
            SignableMessageType::DelegateAction => Self {
                discriminant: NEP_366_META_TRANSACTIONS,
            },
        }
    }
}
