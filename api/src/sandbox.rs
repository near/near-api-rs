use near_api_types::sandbox::StateRecord;

use crate::advanced::{
    RequestBuilder, RpcBuilder,
    sandbox_rpc::{SandboxAction, SimpleSandboxRpc},
};

#[derive(Clone, Debug, Copy)]
pub struct Sandbox;

impl Sandbox {
    pub fn patch_state(state: Vec<StateRecord>) -> RpcBuilder<SimpleSandboxRpc, SimpleSandboxRpc> {
        let rpc: SimpleSandboxRpc = SimpleSandboxRpc {
            action: SandboxAction::PatchState(state),
        };

        RequestBuilder::new(rpc.clone(), (), rpc)
    }

    pub fn fast_forward(height: u64) -> RpcBuilder<SimpleSandboxRpc, SimpleSandboxRpc> {
        let rpc: SimpleSandboxRpc = SimpleSandboxRpc {
            action: SandboxAction::FastForward(height),
        };

        RequestBuilder::new(rpc.clone(), (), rpc)
    }
}
