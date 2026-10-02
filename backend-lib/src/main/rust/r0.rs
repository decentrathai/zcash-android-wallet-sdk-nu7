//! ZIP 316 Revision 0 encodings of Unified Addresses and Unified Viewing Keys.
//!
//! `zcash_keys 0.17` encodes Unified Addresses, UFVKs and UIVKs as ZIP 316 Revision 2
//! (`zu…`/`tu…`, `uvf…`, `uvi…`) unconditionally, and its shielded-only `zu` form drops the
//! transparent receiver. SDK 3.0.0 handed Kotlin the Revision 0 forms (`u1…`, `uview1…`,
//! `uivk1…` and their testnet equivalents), and downstream apps parse those prefixes, so this
//! build keeps returning Revision 0 strings until the upstream SDK makes the switch.
//!
//! Every conversion keeps all data items (receivers or viewing keys) and changes only the
//! revision. A container that carries metadata (such as an expiry height) or that ZIP 316 does
//! not allow in Revision 0 (for example a transparent-only address) has no Revision 0 form and is
//! returned in its Revision 2 encoding unchanged.

use zcash_address::{
    ToAddress, ZcashAddress,
    unified::{self, Container, Encoding, Revision, Uitem},
};
use zcash_client_backend::{
    address::UnifiedAddress,
    keys::{UnifiedFullViewingKey, UnifiedIncomingViewingKey},
};
use zcash_protocol::consensus::{NetworkType, Parameters};

/// Re-encodes a Unified container string as Revision 0 when it has a Revision 0 form, and
/// returns it unchanged otherwise (including when it is already Revision 0 or does not parse as
/// `C`).
fn to_r0<C: Encoding + Container>(encoded: String) -> String {
    let Ok((net, revision, container)) = C::decode(&encoded) else {
        return encoded;
    };
    if revision == Revision::R0 {
        return encoded;
    }

    let mut data_items = Vec::with_capacity(container.items_as_parsed().len());
    for item in container.items_as_parsed() {
        match item {
            Uitem::Data(data) => data_items.push(Uitem::Data(data.clone())),
            Uitem::Metadata(_) => {
                tracing::warn!(
                    "Unified container carries metadata; keeping its Revision 2 encoding"
                );
                return encoded;
            }
        }
    }

    match C::try_from_items(Revision::R0, data_items) {
        Ok(r0) => r0.encode(&net),
        Err(e) => {
            tracing::warn!("Unified container has no Revision 0 form ({e}); keeping Revision 2");
            encoded
        }
    }
}

/// The string form of `ua` that SDK 3.0.0 returned: Revision 0, with every receiver
/// (including a transparent one, which the Revision 2 `zu` form would strip).
pub(crate) fn encode_ua<P: Parameters>(ua: &UnifiedAddress, params: &P) -> String {
    to_r0::<unified::Address>(ua.encode_receiver_preserving(params))
}

/// The string form of `ufvk` that SDK 3.0.0 returned (Revision 0 `uview…`).
pub(crate) fn encode_ufvk<P: Parameters>(ufvk: &UnifiedFullViewingKey, params: &P) -> String {
    to_r0::<unified::Ufvk>(ufvk.encode(params))
}

/// The string form of `uivk` that SDK 3.0.0 returned (Revision 0 `uivk…`).
pub(crate) fn encode_uivk<P: Parameters>(uivk: &UnifiedIncomingViewingKey, params: &P) -> String {
    to_r0::<unified::Uivk>(uivk.encode(params))
}

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

#[cfg(test)]
mod tests {
    use zcash_address::unified::{self, Container, Encoding, Revision, Uitem};
    use zcash_client_backend::{
        address::UnifiedAddress,
        keys::{
            ReceiverRequirement::{Omit, Require},
            UnifiedAddressRequest, UnifiedFullViewingKey, UnifiedSpendingKey,
        },
    };
    use zcash_protocol::consensus::{BlockHeight, Network, NetworkType, Parameters};

    use super::{encode_ua, encode_ufvk, encode_uivk, orchard_only_address, to_r0};
    use crate::test_vectors::{digest, expected, seed};

    fn ufvk_for(name: &str, network: &Network) -> UnifiedFullViewingKey {
        UnifiedSpendingKey::from_seed(network, &seed(name), zip32::AccountId::ZERO)
            .expect("valid seed")
            .to_unified_full_viewing_key()
    }

    fn first_address(
        ufvk: &UnifiedFullViewingKey,
        request: UnifiedAddressRequest,
    ) -> UnifiedAddress {
        ufvk.find_address(zip32::DiversifierIndex::new(), request)
            .expect("an address is derivable")
            .0
    }

    fn network(tag: &str) -> Network {
        match tag {
            "main" => Network::MainNetwork,
            _ => Network::TestNetwork,
        }
    }

    #[test]
    fn strings_match_sdk_3_0_0() {
        let sapling_orchard = UnifiedAddressRequest::custom(Require, Require, Omit).unwrap();
        let cases = ["Alice", "Ben"].into_iter().flat_map(|name| {
            ["main", "test"].into_iter().flat_map(move |tag| {
                ["ua_all", "ua_sapling_orchard", "ufvk", "uivk"].map(|kind| (name, tag, kind))
            })
        });
        for (name, tag, kind) in cases {
            let network = network(tag);
            let ufvk = ufvk_for(name, &network);
            let ours = match kind {
                "ua_all" => encode_ua(
                    &first_address(&ufvk, UnifiedAddressRequest::AllAvailableKeys),
                    &network,
                ),
                "ua_sapling_orchard" => encode_ua(&first_address(&ufvk, sapling_orchard), &network),
                "ufvk" => encode_ufvk(&ufvk, &network),
                "uivk" => encode_uivk(&ufvk.to_unified_incoming_viewing_key(), &network),
                _ => unreachable!(),
            };
            assert_eq!(
                digest(ours.as_bytes()),
                expected(name, tag, kind),
                "{name} {tag} {kind} differs from SDK 3.0.0"
            );
        }
    }

    #[test]
    fn upstream_encoding_differs_from_sdk_3_0_0() {
        // Guards the premise of this module: without it, the strings would change.
        let network = Network::MainNetwork;
        let ufvk = ufvk_for("Ben", &network);
        let ua = first_address(&ufvk, UnifiedAddressRequest::AllAvailableKeys);
        assert_ne!(
            digest(ua.encode(&network).as_bytes()),
            expected("Ben", "main", "ua_all")
        );
        assert_ne!(
            digest(ufvk.encode(&network).as_bytes()),
            expected("Ben", "main", "ufvk")
        );
    }

    #[test]
    fn viewing_keys_round_trip() {
        for (network, ufvk_hrp, uivk_hrp) in [
            (Network::MainNetwork, "uview1", "uivk1"),
            (Network::TestNetwork, "uviewtest1", "uivktest1"),
        ] {
            let ufvk = ufvk_for("Alice", &network);
            let ufvk_r0 = encode_ufvk(&ufvk, &network);
            assert!(ufvk_r0.starts_with(ufvk_hrp), "UFVK is not Revision 0");
            let decoded =
                UnifiedFullViewingKey::decode(&network, &ufvk_r0).expect("R0 UFVK decodes");
            assert!(
                decoded.encode(&network) == ufvk.encode(&network),
                "UFVK content changed"
            );

            let uivk = ufvk.to_unified_incoming_viewing_key();
            let uivk_r0 = encode_uivk(&uivk, &network);
            assert!(uivk_r0.starts_with(uivk_hrp), "UIVK is not Revision 0");
            let (net, revision, r0) = unified::Uivk::decode(&uivk_r0).expect("R0 UIVK decodes");
            let (_, _, r2) =
                unified::Uivk::decode(&uivk.encode(&network)).expect("R2 UIVK decodes");
            assert_eq!((net, revision), (network.network_type(), Revision::R0));
            assert!(r0.items() == r2.items(), "UIVK items changed");
        }
    }

    #[test]
    fn revision_0_input_is_unchanged_and_garbage_passes_through() {
        let network = Network::TestNetwork;
        let ua = first_address(
            &ufvk_for("Ben", &network),
            UnifiedAddressRequest::AllAvailableKeys,
        );
        let r0 = encode_ua(&ua, &network);
        assert!(to_r0::<unified::Address>(r0.clone()) == r0);
        assert_eq!(
            to_r0::<unified::Address>("not an address".to_owned()),
            "not an address"
        );
    }

    #[test]
    fn metadata_keeps_revision_2() {
        let network = Network::MainNetwork;
        let ua = first_address(
            &ufvk_for("Ben", &network),
            UnifiedAddressRequest::AllAvailableKeys,
        );
        let expiring = UnifiedAddress::from_receivers(
            ua.orchard().copied(),
            ua.sapling().copied(),
            ua.transparent().copied(),
            Some(BlockHeight::from_u32(5_000_000)),
            None,
        )
        .expect("valid receivers");
        let (_, revision, parsed) =
            unified::Address::decode(&encode_ua(&expiring, &network)).expect("decodes");
        assert_eq!(revision, Revision::R2);
        assert!(
            parsed
                .items_as_parsed()
                .iter()
                .any(|i| matches!(i, Uitem::Metadata(_))),
            "expiry metadata must survive"
        );
    }

    #[test]
    fn orchard_only_address_is_revision_0() {
        for network in [Network::MainNetwork, Network::TestNetwork] {
            let receiver = ufvk_for("Alice", &network)
                .orchard()
                .expect("Orchard FVK")
                .address_at(0u32, orchard::keys::Scope::Internal);
            let encoded = orchard_only_address(network.network_type(), &receiver).encode();
            let prefix = match network.network_type() {
                NetworkType::Main => "u1",
                _ => "utest1",
            };
            assert!(encoded.starts_with(prefix), "not a Revision 0 address");
            let (_, revision, ua) = unified::Address::decode(&encoded).expect("decodes");
            assert_eq!(revision, Revision::R0);
            assert!(
                ua.items() == vec![unified::Receiver::Orchard(receiver.to_raw_address_bytes())],
                "receiver changed"
            );
        }
    }
}
