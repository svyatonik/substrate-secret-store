// TODO: track account indices

// Copyright 2020 Parity Technologies (UK) Ltd.
// This file is part of Parity-Bridge.

// Parity-Bridge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity-Bridge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity-Bridge.  If not, see <http://www.gnu.org/licenses/>.

// https://github.com/scs/substrate-api-client/blob/master/src/examples/example_event_callback.rs

use codec::{Decode, Encode};
use sp_core::crypto::Pair;
use sp_runtime::traits::IdentifyAccount;

/// System::events storage key. Calculated as:
/// twox_128(b"System").to_vec() ++ twox_128(b"Events").to_vec()
const SYSTEM_EVENTS_KEY: &'static str = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";

/// All possible errors that can occur during interacting with Substrate node.
#[derive(Debug)]
pub enum Error {
	/// Client creation has failed.
	ClientCreationFailed(jsonrpsee::ws::WsNewDnsError),
	/// Request has failed.
	RequestFailed(jsonrpsee::client::RequestError),
	/// Response decode has failed.
	DecodeFailed(codec::Error),
}

/// Substrate client type.
#[derive(Clone)]
pub struct Client {
	/// Substrate RPC client.
	rpc_client: jsonrpsee::Client,
	/// Transactions signer.
	signer: sp_core::sr25519::Pair,
	/// Genesis block hash.
	genesis_hash: sp_core::H256,
	/// Runtime version.
	runtime_version: u32,
}

impl Client {
	/// Create new client.
	pub async fn new(
		uri: &str,
		signer: sp_core::sr25519::Pair,
	) -> Result<Self, Error> {
		let rpc_client = jsonrpsee::ws_client(uri).await.map_err(Error::ClientCreationFailed)?;
		let genesis_hash = rpc_client.request(
			"chain_getBlockHash",
			jsonrpsee::core::common::Params::Array(vec![
				serde_json::to_value(0u32).unwrap(),
			]),
		).await.map_err(Error::RequestFailed)?;
		/*let finalized_header_hash: sp_core::H256 = rpc_client.request(
			"chain_getFinalizedHead",
			jsonrpsee::core::common::Params::None,
		).await.map_err(Error::RequestFailed)?;
		let finalized_header: Option<node_runtime::Header> = rpc_client.request(
			"chain_getHeader",
			jsonrpsee::core::common::Params::Array(vec![
				serde_json::to_value(finalized_header_hash).unwrap(),
			]),
		).await.map_err(Error::RequestFailed)?;
		let finalized_header = finalized_header.ok_or(Error::MissingFinalizedHeader)?;*/
		let runtime_version: sp_version::RuntimeVersion = rpc_client.request(
			"state_getRuntimeVersion",
			jsonrpsee::core::common::Params::None,
		).await.map_err(Error::RequestFailed)?;

		Ok(Client {
			rpc_client,
			signer,
			genesis_hash,
			runtime_version: runtime_version.spec_version,
		})
	}

	/// Subscribe to new blocks.
	pub async fn subscribe_finalized_heads(&self) -> Result<jsonrpsee::client::Subscription<node_runtime::Header>, Error> {
		self.rpc_client.subscribe(
			"chain_subscribeFinalizedHeads",
			jsonrpsee::core::common::Params::None,
			"chain_unsubscribeFinalizedHeads",
		).await.map_err(Error::RequestFailed)
	}

	/// Read events of the header.
	pub async fn header_events(&self, hash: sp_core::H256) -> Result<Vec<frame_system::EventRecord<node_runtime::Event, sp_core::H256>>, Error> {
		let events_storage: Option<sp_core::Bytes> = self.rpc_client.request(
			"state_getStorage",
			jsonrpsee::core::common::Params::Array(vec![
				serde_json::to_value(format!("0x{}", SYSTEM_EVENTS_KEY)).unwrap(),
				serde_json::to_value(hash).unwrap(),
			]),
		).await.map_err(Error::RequestFailed)?;
		match events_storage {
			Some(events_storage) => Decode::decode(&mut &events_storage[..])
				.map_err(Error::DecodeFailed),
			None => Ok(Vec::new())
		}
	}

	/// Call runtime method.
	pub async fn call_runtime_method<Ret: Decode>(
		&self,
		hash: sp_core::H256,
		method: &'static str,
		arguments: Vec<Vec<u8>>,
	) -> Result<Ret, Error> {
		self.rpc_client.request(
			"state_call",
			jsonrpsee::core::common::Params::Array(vec![
				serde_json::to_value(method).unwrap(),
				serde_json::to_value(arguments.encode()).unwrap(),
				serde_json::to_value(hash).unwrap(),
			]),
		)
		.await
		.map_err(Error::RequestFailed)
		.and_then(|ret: sp_core::Bytes| Ret::decode(&mut &ret.0[..]).map_err(Error::DecodeFailed))
	}

	/// Submit runtime transaction.
	pub async fn submit_transaction(&self, call: node_runtime::Call) -> Result<sp_core::H256, Error> {
		let index = self.next_account_index().await?;
		let transaction = create_transaction(
			call,
			&self.signer,
			index,
			self.genesis_hash,
			self.runtime_version,
		);
		self.rpc_client.request(
			"author_submitExtrinsic",
			jsonrpsee::core::common::Params::Array(vec![
				serde_json::to_value(transaction.encode()).unwrap(),
			]),
		).await.map_err(Error::RequestFailed)
	}

	/// Get substrate account nonce.
	async fn next_account_index(&self) -> Result<node_primitives::Index, Error> {
		use sp_core::crypto::Ss58Codec;

		let account_id: node_primitives::AccountId = self.signer.public().as_array_ref().clone().into();
		self.rpc_client.request(
			"system_accountNextIndex",
			jsonrpsee::core::common::Params::Array(vec![
				serde_json::to_value(account_id.to_ss58check()).unwrap(),
			]),
		).await.map_err(Error::RequestFailed)
	}
}

/// Encode runtime transaction.
fn create_transaction(
	call: node_runtime::Call,
	signer: &sp_core::sr25519::Pair,
	index: node_primitives::Index,
	genesis_hash: sp_core::H256,
	runtime_version: u32,
) -> node_runtime::UncheckedExtrinsic {
	let extra = |i: node_primitives::Index, f: node_primitives::Balance| {
		(
			frame_system::CheckVersion::<node_runtime::Runtime>::new(),
			frame_system::CheckGenesis::<node_runtime::Runtime>::new(),
			frame_system::CheckEra::<node_runtime::Runtime>::from(sp_runtime::generic::Era::Immortal),
			frame_system::CheckNonce::<node_runtime::Runtime>::from(i),
			frame_system::CheckWeight::<node_runtime::Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<node_runtime::Runtime>::from(f),
			Default::default(),
		)
	};
	let raw_payload = node_runtime::SignedPayload::from_raw(
		call,
		extra(index, 0),
		(
			runtime_version,
			genesis_hash,
			genesis_hash,
			(),
			(),
			(),
			(),
		),
	);
	let signature = raw_payload.using_encoded(|payload| signer.sign(payload));
	let signer: sp_runtime::MultiSigner = signer.public().into();
	let (function, extra, _) = raw_payload.deconstruct();

	node_runtime::UncheckedExtrinsic::new_signed(
		function,
		signer.into_account().into(),
		signature.into(),
		extra,
	)
}
