//! Ground truth for the NU7 SDK port's Revision 0 shim: the strings SDK 3.0.0 (zcash_keys 0.16.1)
//! returns for the public Alice/Ben test wallets, recorded as 16-hex SHA-256 prefixes only.
//! Usage: groundtruth-016 <path to WalletFixture.kt> > digests.txt
use sha2::{Digest, Sha256};
use zcash_keys::keys::{ReceiverRequirement::*, UnifiedAddressRequest, UnifiedSpendingKey};
use zcash_address::unified::{self, Container, Encoding, Item as _};
use zcash_protocol::consensus::{Network, NetworkConstants};
use zip32::{ChildIndex, registered::{PathElement, SecretKey}};

/// The SDK 3.0.0 JNI derivations (deriveAccountMetadataKeyFromSeed, derivePrivateUseMetadataKey).
const SUBJECT: &[u8] = b"zchat-nu7-groundtruth";

fn literals(t: &str) -> Vec<&str> { t.split('"').skip(1).step_by(2).collect() }

fn seed_phrase(fixture: &str, name: &str) -> String {
    let s = &fixture[fixture.find(&format!("data object {name}")).unwrap()..];
    let a = s.find("seedPhrase: String").unwrap();
    let b = s.find("getBirthday").unwrap();
    literals(&s[a..b]).concat()
}

fn d(s: &str) -> String { db(s.as_bytes()) }
fn db(b: &[u8]) -> String { hex::encode(&Sha256::digest(b)[..8]) }

fn main() {
    let fixture = std::fs::read_to_string(std::env::args().nth(1).expect("fixture path")).unwrap();
    for name in ["Alice", "Ben"] {
        let m = bip0039::Mnemonic::<bip0039::English>::from_phrase(seed_phrase(&fixture, name)).unwrap();
        let seed = m.to_seed("");
        for (net, tag) in [(Network::MainNetwork, "main"), (Network::TestNetwork, "test")] {
            let usk = UnifiedSpendingKey::from_seed(&net, &seed, zip32::AccountId::ZERO).unwrap();
            let ufvk = usk.to_unified_full_viewing_key();
            let (ua_all, _) = ufvk
                .find_address(zip32::DiversifierIndex::new(), UnifiedAddressRequest::AllAvailableKeys)
                .unwrap();
            let so = UnifiedAddressRequest::custom(Require, Require, Omit).unwrap();
            let (ua_so, _) = ufvk.find_address(zip32::DiversifierIndex::new(), so).unwrap();
            println!("{name} {tag} ua_all {}", d(&ua_all.encode(&net)));
            println!("{name} {tag} ua_sapling_orchard {}", d(&ua_so.encode(&net)));
            println!("{name} {tag} ufvk {}", d(&ufvk.encode(&net)));
            println!("{name} {tag} uivk {}", d(&ufvk.to_unified_incoming_viewing_key().encode(&net)));
            let ak = SecretKey::from_subpath(b"MetadataKeys", &seed, 325, &[
                PathElement::new(ChildIndex::hardened(net.coin_type()), &[]),
                PathElement::new(ChildIndex::hardened(0), &[]),
            ]).unwrap();
            println!("{name} {tag} mk_account_sk {}", db(ak.data()));
            println!("{name} {tag} mk_account_c {}", db(ak.chain_code().as_bytes()));
            let inherent = ak.derive_child_with_tag(ChildIndex::hardened(0), &[])
                .derive_child_with_tag(ChildIndex::PRIVATE_USE, SUBJECT);
            println!("{name} {tag} mk_inherent {}", db(inherent.data()));
            let (_, u) = unified::Ufvk::decode(&ufvk.encode(&net)).unwrap();
            let ext: Vec<String> = u.items().into_iter().map(|item| {
                db(ak.derive_child_with_tag(ChildIndex::hardened(1), &[])
                    .derive_child_with_tag(ChildIndex::hardened(0), &item.typed_encoding())
                    .derive_child_with_tag(ChildIndex::PRIVATE_USE, SUBJECT).data())
            }).collect();
            println!("{name} {tag} mk_external {}", ext.join(","));
        }
    }
}
