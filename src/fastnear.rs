use std::collections::BTreeSet;

use near_primitives::types::AccountId;
use serde::de::DeserializeOwned;

use crate::errors::FastNearError;

#[derive(Debug, serde::Deserialize)]
pub struct StakingPool {
    pool_id: near_primitives::types::AccountId,
}

#[derive(Debug, serde::Deserialize)]
pub struct StakingResponse {
    pools: Vec<StakingPool>,
}

pub struct FastNearBuilder<T: DeserializeOwned, PostProcessed> {
    query: String,
    post_process: Box<dyn Fn(T) -> PostProcessed + Send + Sync>,
    _response: std::marker::PhantomData<T>,
}

impl<T: DeserializeOwned> FastNearBuilder<T, T> {
    pub fn new(query: String) -> Self {
        Self {
            query,
            post_process: Box::new(|response| response),
            _response: Default::default(),
        }
    }
}

impl<T: DeserializeOwned, PostProcessed> FastNearBuilder<T, PostProcessed> {
    pub fn with_postprocess<F>(query: String, func: F) -> Self
    where
        F: Fn(T) -> PostProcessed + Send + Sync + 'static,
    {
        Self {
            query,
            post_process: Box::new(func),
            _response: Default::default(),
        }
    }

    pub async fn fetch_from_url(self, url: url::Url) -> Result<PostProcessed, FastNearError> {
        let request = reqwest::get(url.join(&self.query)?).await?;
        Ok((self.post_process)(request.json().await?))
    }

    pub async fn fetch_from_mainnet(self) -> Result<PostProcessed, FastNearError> {
        match crate::config::NetworkConfig::mainnet().fastnear_url {
            Some(url) => self.fetch_from_url(url).await,
            None => Err(FastNearError::FastNearUrlIsNotDefined),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FastNear {}

impl FastNear {
    pub async fn pools_delegated_by(
        &self,
        account_id: &AccountId,
    ) -> Result<FastNearBuilder<StakingResponse, BTreeSet<AccountId>>, FastNearError> {
        let query_builder = FastNearBuilder::with_postprocess(
            format!("v1/account/{}/staking", account_id),
            |response: StakingResponse| {
                response
                    .pools
                    .into_iter()
                    .map(|pool| pool.pool_id)
                    .collect()
            },
        );

        Ok(query_builder)
    }
}
