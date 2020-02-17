// TODO: replace block_on with ThreadPool + channel (we can't use async everywhere :/)

use std::{
	collections::BTreeSet,
	ops::Range,
};
use log::error;
use parity_secretstore_primitives::{
	Address, KeyServerId, ServerKeyId,
};
use parity_secretstore_substrate_service::{
	Blockchain, BlockchainServiceTask, MaybeSecretStoreEvent,
};
use crate::{
	substrate_client::Client,
};

/// Substrate-based blockhain that runs SecretStore module.
pub struct SecretStoreBlockchain {
	/// RPC client that can call RPC on full (presumably archive node) that
	/// is synching the blockhain.
	client: Client,
}

/// Runtime event wrapper.
pub struct SecretStoreEvent(crate::runtime::Event);

impl SecretStoreBlockchain {
	///
	pub fn new(client: Client) -> SecretStoreBlockchain {
		SecretStoreBlockchain {
			client,
		}
	}
}

impl Blockchain for SecretStoreBlockchain {
	type BlockHash = crate::runtime::BlockHash;
	type Event = SecretStoreEvent;
	type BlockEvents = Vec<SecretStoreEvent>;
	type PendingEvents = Vec<SecretStoreEvent>;

	fn block_events(&self, block_hash: Self::BlockHash) -> Self::BlockEvents {
		let events = futures::executor::block_on(
			self.client.header_events(block_hash)
		);

		match events {
			Ok(events) => events
				.into_iter()
				.map(|event| SecretStoreEvent(event.event))
				.collect(),
			Err(error) => {
				error!(
					target: "secretstore",
					"Failed to read block {} events: {:?}",
					block_hash,
					error,
				);

				return Vec::new();
			}
		}
	}

	fn current_key_servers_set(&self) -> BTreeSet<KeyServerId> {
		unimplemented!()
	}

	fn server_key_generation_tasks(
		&self,
		block_hash: Self::BlockHash,
		range: Range<usize>,
	) -> Result<Self::PendingEvents, String> {
		let events: Vec<substrate_secret_store_runtime::Event> = futures::executor::block_on(async {
			self.client.call_runtime_method(
				block_hash,
				"SecretStoreServiceApi_server_key_generation_tasks",
				serialize_range(range),
			).await
		}).map_err(|error| format!("{:?}", error))?;
		Ok(events
			.into_iter()
			.map(|event| SecretStoreEvent(crate::runtime::Event::substrate_secret_store_runtime(event)))
			.collect())
	}

	fn is_server_key_generation_response_required(
		&self,
		_key_id: ServerKeyId,
		_key_server_id: KeyServerId,
	) -> Result<bool, String> {
		unimplemented!()
	}

	fn server_key_retrieval_tasks(
		&self,
		_block_hash: Self::BlockHash,
		_range: Range<usize>,
	) -> Result<Self::PendingEvents, String> {
		unimplemented!()
	}

	fn is_server_key_retrieval_response_required(
		&self,
		_key_id: ServerKeyId,
		_key_server_id: KeyServerId,
	) -> Result<bool, String> {
		unimplemented!()
	}

	fn document_key_store_tasks(
		&self,
		_block_hash: Self::BlockHash,
		_range: Range<usize>,
	) -> Result<Self::PendingEvents, String> {
		unimplemented!()
	}

	fn is_document_key_store_response_required(
		&self,
		_key_id: ServerKeyId,
		_key_server_id: KeyServerId,
	) -> Result<bool, String> {
		unimplemented!()
	}

	fn document_key_shadow_retrieval_tasks(
		&self,
		_block_hash: Self::BlockHash,
		_range: Range<usize>,
	) -> Result<Self::PendingEvents, String> {
		unimplemented!()
	}

	fn is_document_key_shadow_retrieval_response_required(
		&self,
		_key_id: ServerKeyId,
		_requester: Address,
		_key_server_id: KeyServerId,
	) -> Result<bool, String> {
		unimplemented!()
	}
}

impl MaybeSecretStoreEvent for SecretStoreEvent {
	fn as_secret_store_event(self) -> Option<substrate_secret_store_runtime::Event> {
		match self.0 {
			crate::runtime::Event::substrate_secret_store_runtime(event) => Some(event),
			_ => None,
		}
	}
}

fn serialize_range(range: Range<usize>) -> Vec<Vec<u8>> {
	unimplemented!()
}