use std::sync::Arc;
use ethcore_secretstore::{Error, KeyServer, SigningKeyPair};

/// Start secret store key server.
pub fn start(
	_substrate_client: crate::substrate_client::Client,
) -> Result<Box<dyn KeyServer>, Error> {
	unimplemented!()
}
