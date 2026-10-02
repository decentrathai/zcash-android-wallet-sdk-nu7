//! ZIP 316 Revision 0 encodings of Unified Addresses.
//!
//! `zcash_keys 0.17` encodes Unified Addresses as ZIP 316 Revision 2 (`zu…`) unconditionally.
//! SDK 3.0.0 built the account's own internal migration destination as a Revision 0 address
//! (`u1…`), and this build keeps doing so.

use zcash_address::{
    ToAddress, ZcashAddress,
    unified::{self, Encoding, Revision, Uitem},
};
use zcash_protocol::consensus::NetworkType;

/// The Revision 0 Unified Address holding only the given Orchard receiver, as SDK 3.0.0
/// built it for the account's own internal migration destination.
pub(crate) fn orchard_only_address(net: NetworkType, receiver: &orchard::Address) -> ZcashAddress {
    let ua = unified::Address::try_from_items(
        Revision::R0,
        vec![Uitem::Data(unified::Receiver::Orchard(
            receiver.to_raw_address_bytes(),
        ))],
    )
    .expect("an Orchard receiver alone is a valid Revision 0 Unified Address");
    ZcashAddress::from_unified(net, ua)
}
