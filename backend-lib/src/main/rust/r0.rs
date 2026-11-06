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
//!
//! `clippy.toml` disallows the upstream Revision 2 encoders everywhere else in this crate; this
//! module is the one place that calls them.

#![allow(clippy::disallowed_methods)]

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
    // zcash_keys 0.17.0 (stable) encodes at Revision 0 whenever Revision 0 can represent the
    // address, with every receiver; `to_r0` is then a no-op and stays as the guard.
    to_r0::<unified::Address>(ua.to_zcash_address(params.network_type()).encode())
}

/// The string form of `ufvk` that SDK 3.0.0 returned (Revision 0 `uview…`).
pub(crate) fn encode_ufvk<P: Parameters>(ufvk: &UnifiedFullViewingKey, params: &P) -> String {
    // zcash_keys 0.17.0: Revision 0 when representable, else Revision 2; an error means neither
    // revision can encode the key, which no key this SDK derives or imports can be.
    to_r0::<unified::Ufvk>(ufvk.encode(params).expect("a UFVK has a ZIP 316 encoding"))
}

/// The string form of `uivk` that SDK 3.0.0 returned (Revision 0 `uivk…`).
pub(crate) fn encode_uivk<P: Parameters>(uivk: &UnifiedIncomingViewingKey, params: &P) -> String {
    to_r0::<unified::Uivk>(uivk.encode(params).expect("a UIVK has a ZIP 316 encoding"))
}

/// The Revision 0 form of a Unified Address, UFVK or UIVK string; any other string (a
/// transparent, Sapling or TEX address, one that is already Revision 0, or one with no Revision 0
/// form) is returned unchanged.
///
/// This is the conversion for address strings read back from the wallet database:
/// `zcash_client_sqlite` stores the addresses it hands out in their receiver-preserving Revision 2
/// form (`tu…` with a transparent receiver, `zu…` without), while rows written by SDK 3.0.0 are
/// Revision 0. Stored rows are never rewritten; only the string handed to the app is converted.
pub(crate) fn to_revision_0(encoded: String) -> String {
    to_r0::<unified::Uivk>(to_r0::<unified::Ufvk>(to_r0::<unified::Address>(encoded)))
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

    use super::{encode_ua, encode_ufvk, encode_uivk, orchard_only_address, to_r0, to_revision_0};
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
        // Stable zcash_keys 0.17.0 encodes at Revision 0 when it can: its default strings are
        // SDK 3.0.0's again (the pre-releases differed; that was why this module exists).
        assert_eq!(
            digest(ua.to_zcash_address(network.network_type()).encode().as_bytes()),
            expected("Ben", "main", "ua_all")
        );
        assert_eq!(
            digest(ufvk.encode(&network).expect("UFVK encodes").as_bytes()),
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
                decoded.encode(&network).expect("UFVK encodes")
                    == ufvk.encode(&network).expect("UFVK encodes"),
                "UFVK content changed"
            );

            let uivk = ufvk.to_unified_incoming_viewing_key();
            let uivk_r0 = encode_uivk(&uivk, &network);
            assert!(uivk_r0.starts_with(uivk_hrp), "UIVK is not Revision 0");
            let (net, revision, r0) = unified::Uivk::decode(&uivk_r0).expect("R0 UIVK decodes");
            let (_, _, r2) =
                unified::Uivk::decode(&uivk.encode_revision(&network, Revision::R2).expect("R2 encodes"))
                    .expect("R2 UIVK decodes");
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

    /// What the wallet database stores for these keys and addresses (Revision 2) reads back as
    /// the string SDK 3.0.0 returned.
    #[test]
    fn stored_revision_2_strings_convert_to_sdk_3_0_0_strings() {
        let sapling_orchard = UnifiedAddressRequest::custom(Require, Require, Omit).unwrap();
        for name in ["Alice", "Ben"] {
            for tag in ["main", "test"] {
                let network = network(tag);
                let ufvk = ufvk_for(name, &network);
                let all = first_address(&ufvk, UnifiedAddressRequest::AllAvailableKeys);
                let shielded = first_address(&ufvk, sapling_orchard);
                let uivk = ufvk.to_unified_incoming_viewing_key();
                for (kind, stored) in [
                    // `tu…`: the stored form of an address with a transparent receiver.
                    ("ua_all", all.to_zcash_address_revision(network.network_type(), Revision::R2).expect("R2 encodes").encode()),
                    // `zu…`: the stored form of a shielded-only address.
                    (
                        "ua_sapling_orchard",
                        shielded.to_zcash_address_revision(network.network_type(), Revision::R2).expect("R2 encodes").encode(),
                    ),
                    ("ufvk", ufvk.encode_revision(&network, Revision::R2).expect("R2 encodes")),
                    ("uivk", uivk.encode_revision(&network, Revision::R2).expect("R2 encodes")),
                ] {
                    let (_, revision, _) = match kind {
                        "ufvk" => unified::Ufvk::decode(&stored).map(|(n, r, _)| (n, r, ())),
                        "uivk" => unified::Uivk::decode(&stored).map(|(n, r, _)| (n, r, ())),
                        _ => unified::Address::decode(&stored).map(|(n, r, _)| (n, r, ())),
                    }
                    .expect("stored string decodes");
                    assert_eq!(revision, Revision::R2, "{name} {tag} {kind}: premise");
                    let read_back = to_revision_0(stored);
                    assert_eq!(
                        digest(read_back.as_bytes()),
                        expected(name, tag, kind),
                        "{name} {tag} {kind} differs from SDK 3.0.0"
                    );
                    // Converting twice changes nothing: Revision 0 rows read back as they are.
                    assert!(to_revision_0(read_back.clone()) == read_back);
                }
            }
        }
    }

    #[test]
    fn other_strings_are_returned_unchanged() {
        let network = Network::MainNetwork;
        let ufvk = ufvk_for("Alice", &network);
        let ua = first_address(&ufvk, UnifiedAddressRequest::AllAvailableKeys);
        let transparent = zcash_client_backend::encoding::AddressCodec::encode(
            ua.transparent().expect("transparent receiver"),
            &network,
        );
        let sapling =
            zcash_client_backend::address::Address::from(*ua.sapling().expect("Sapling receiver"))
                .encode(&network);
        for other in [
            transparent.as_str(),
            sapling.as_str(),
            "",
            "not an address",
            "zu1",
            "tu1qqqq",
        ] {
            assert!(
                to_revision_0(other.to_owned()) == other,
                "a non-unified string changed"
            );
        }
    }

    /// A wallet database holding both kinds of rows: addresses handed out by this build (stored
    /// as Revision 2) and the same rows as SDK 3.0.0 left them (Revision 0). Both must read back
    /// as the one string the SDK returns for that address, including through
    /// `v_tx_outputs.to_address`, the column the Kotlin layer reads.
    #[test]
    fn mixed_database_reads_back_one_string_per_address() {
        use zcash_client_backend::data_api::testing::{AddressType, TestBuilder};
        use zcash_client_backend::data_api::{Account as _, WalletRead, WalletWrite};
        use zcash_client_sqlite::testing::{BlockCache, db::TestDbFactory};
        use zcash_primitives::block::BlockHash;
        use zcash_protocol::value::Zatoshis;

        let mut st = TestBuilder::new()
            .with_data_store_factory(TestDbFactory::default())
            .with_block_cache(BlockCache::new())
            .with_account_from_sapling_activation(BlockHash([0; 32]))
            .build();
        let network = *st.network();
        let account = st.test_account().expect("test account").id();
        // The wallet hands out addresses only once it knows the chain height.
        let (first_height, _) = st.generate_empty_block();
        st.scan_cached_blocks(first_height, 1);
        let rows_equal_to = |conn: &rusqlite::Connection, value: &str| -> Vec<String> {
            let mut stmt = conn
                .prepare("SELECT address FROM addresses WHERE address = ?1")
                .unwrap();
            stmt.query_map([value], |row| row.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };

        // The account's default address, before any other address is handed out.
        let default_address = st
            .wallet()
            .get_last_generated_address_matching(account, UnifiedAddressRequest::AllAvailableKeys)
            .unwrap()
            .expect("default address");

        // Hand out addresses as `getNextAvailableAddress` does: Sapling+Orchard (receiver flags
        // 12) and one with every receiver.
        let sapling_orchard = UnifiedAddressRequest::custom(Require, Require, Omit).unwrap();
        for request in [sapling_orchard, UnifiedAddressRequest::AllAvailableKeys] {
            let (ua, _) = st
                .wallet_mut()
                .get_next_available_address(account, request)
                .unwrap()
                .expect("an address is available");
            let returned = encode_ua(&ua, &network);
            // Stable zcash_client_sqlite 0.23.0 stores the Revision 0 form when it exists, so the
            // stored row and the returned string are the same (the pre-release builds stored R2).
            let stored = ua.to_zcash_address(network.network_type()).encode();
            assert_eq!(stored, returned, "the stored form is Revision 0 again");
            assert_eq!(
                rows_equal_to(st.wallet().conn(), &stored),
                vec![stored.clone()]
            );
            assert!(to_revision_0(stored.clone()) == returned, "Revision 2 row");

            // The same row as SDK 3.0.0 wrote it.
            let updated = st
                .wallet_mut()
                .conn_mut()
                .execute(
                    "UPDATE addresses SET address = ?1 WHERE address = ?2",
                    [&returned, &stored],
                )
                .unwrap();
            assert_eq!(updated, 1);
            let rows = rows_equal_to(st.wallet().conn(), &returned);
            assert_eq!(rows.len(), 1);
            assert!(to_revision_0(rows[0].clone()) == returned, "Revision 0 row");
        }

        // A note received at the account's default address, read through the view.
        let dfvk = st.test_account_sapling().expect("Sapling key").clone();
        let (height, _, _) = st.generate_next_block(
            &dfvk,
            AddressType::DefaultExternal,
            Zatoshis::const_from_u64(50_000),
        );
        st.scan_cached_blocks(height, 1);
        let to_addresses: Vec<String> = {
            let mut stmt = st
                .wallet()
                .conn()
                .prepare("SELECT to_address FROM v_tx_outputs WHERE to_address IS NOT NULL")
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        assert_eq!(
            to_addresses.len(),
            1,
            "the received output names its address"
        );
        assert!(
            to_revision_0(to_addresses[0].clone()) == encode_ua(&default_address, &network),
            "v_tx_outputs.to_address reads back as the address the SDK returns"
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
