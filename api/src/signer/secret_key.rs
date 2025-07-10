use tracing::{instrument, trace};

use near_types::{AccountId, Convert, PublicKey, SecretKey};

use crate::errors::SignerError;

use super::SignerTrait;

const SECRET_KEY_SIGNER_TARGET: &str = "near_api::signer::secret_key";

#[derive(Debug, Clone)]
pub struct SecretKeySigner {
    secret_key: SecretKey,
    public_key: PublicKey,
}

#[async_trait::async_trait]
impl SignerTrait for SecretKeySigner {
    #[instrument(skip(self))]
    async fn get_secret_key(
        &self,
        signer_id: &AccountId,
        public_key: &PublicKey,
    ) -> Result<SecretKey, SignerError> {
        trace!(target: SECRET_KEY_SIGNER_TARGET, "returning with secret key");
        Ok(self.secret_key.clone())
    }

    #[instrument(skip(self))]
    fn get_public_key(&self) -> Result<PublicKey, SignerError> {
        Ok(self.public_key.clone())
    }
}

impl SecretKeySigner {
    pub fn new(secret_key: SecretKey) -> Self {
        let public_key = secret_key.public_key();
        Self {
            secret_key,
            public_key: Convert(public_key).into(),
        }
    }
}
