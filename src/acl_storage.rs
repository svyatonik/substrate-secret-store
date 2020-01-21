use codec::Encode;
use parking_lot::RwLock;
use sp_core::H256;
use ss_primitives::secret_store::{AclStorage, ServerKeyId, RequesterId};
use crate::substrate_client::Client;

pub struct OnChainAclStorage {
	client: Client,
	data: RwLock<OnChainAclStorageData>,
}

struct OnChainAclStorageData {
	best_block: Option<(u32, H256)>,
}

impl OnChainAclStorage {
	pub fn new(client: Client) -> Self {
		OnChainAclStorage {
			client,
			data: RwLock::new(OnChainAclStorageData {
				best_block: None,
			}),
		}
	}

	pub fn set_best_block(&self, best_block: (u32, H256)) {
		self.data.write().best_block = Some(best_block);
	}
}

impl AclStorage for OnChainAclStorage {
	fn check(&self, server_key_id: ServerKeyId, requester_id: RequesterId) -> Result<bool, String> {
		let best_block = self.data.read().best_block.ok_or_else(|| "disconnected".to_owned())?;
		futures::executor::block_on(async {
			self.client.call_runtime_method(
				best_block.1,
				"SecretStoreAclApi_check",
				vec![server_key_id.encode(), requester_id.encode()],
			).await.map_err(|err| format!("{:?}", err))
		})
	}
}
