use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::chain::ChainInfo;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Random beacon as generated by drand.
/// These can be chained or unchained, and should be verifiable against a chain.
pub enum RandomnessBeacon {
    ChainedBeacon(ChainedBeacon),
    UnchainedBeacon(UnchainedBeacon),
}

impl RandomnessBeacon {
    pub fn verify(&self, info: ChainInfo) -> Result<bool> {
        if self.scheme_id() != info.scheme_id() {
            return Ok(false);
        }

        let signature_verify =
            crate::bls_signatures::verify(&self.signature(), &self.message()?, &info.public_key())?;

        let mut hasher = Sha256::new();
        hasher.update(self.signature());
        let randomness = hasher.finalize().to_vec();
        let randomness_verify = randomness == self.randomness();

        Ok(signature_verify && randomness_verify)
    }

    pub fn round(&self) -> u64 {
        match self {
            Self::ChainedBeacon(chained) => chained.round,
            Self::UnchainedBeacon(unchained) => unchained.round,
        }
    }

    pub fn randomness(&self) -> Vec<u8> {
        match self {
            Self::ChainedBeacon(chained) => chained.randomness.clone(),
            Self::UnchainedBeacon(unchained) => unchained.randomness.clone(),
        }
    }

    pub fn scheme_id(&self) -> String {
        match self {
            Self::ChainedBeacon(_) => "pedersen-bls-chained",
            Self::UnchainedBeacon(unchained) => {
                if unchained.signature.len() == 48 {
                    "bls-unchained-on-g1"
                } else {
                    "pedersen-bls-unchained"
                }
            }
        }
        .to_string()
    }

    pub fn is_signature_on_g1(&self) -> bool {
        self.scheme_id().contains("on-g1")
    }

    pub fn is_unchained(&self) -> bool {
        self.scheme_id().contains("unchained")
    }

    pub fn signature(&self) -> Vec<u8> {
        match self {
            Self::ChainedBeacon(chained) => chained.signature.clone(),
            Self::UnchainedBeacon(unchained) => unchained.signature.clone(),
        }
    }
}

impl Message for RandomnessBeacon {
    fn message(&self) -> Result<Vec<u8>> {
        match self {
            Self::ChainedBeacon(chained) => chained.message(),
            Self::UnchainedBeacon(unchained) => unchained.message(),
        }
    }
}

impl From<ChainedBeacon> for RandomnessBeacon {
    fn from(b: ChainedBeacon) -> Self {
        Self::ChainedBeacon(b)
    }
}

impl From<UnchainedBeacon> for RandomnessBeacon {
    fn from(b: UnchainedBeacon) -> Self {
        Self::UnchainedBeacon(b)
    }
}

/// Package item to be validated against a BLS signature given a public key.
trait Message {
    fn message(&self) -> Result<Vec<u8>>;
}

#[derive(Debug, Serialize, Deserialize)]
/// Chained drand beacon.
/// Each signature depends on the previous one, as well as on the round.
pub struct ChainedBeacon {
    round: u64,
    #[serde(with = "hex::serde")]
    randomness: Vec<u8>,
    #[serde(with = "hex::serde")]
    signature: Vec<u8>,
    #[serde(with = "hex::serde")]
    previous_signature: Vec<u8>,
}

impl ChainedBeacon {
    pub fn new(
        round: u64,
        randomness: Vec<u8>,
        signature: Vec<u8>,
        previous_signature: Vec<u8>,
    ) -> Self {
        Self {
            round,
            randomness,
            signature,
            previous_signature,
        }
    }
}

impl Message for ChainedBeacon {
    fn message(&self) -> Result<Vec<u8>> {
        let mut buf = [0; 96 + 8];
        let (signature_buf, round_buf) = buf.split_at_mut(96);

        signature_buf.clone_from_slice(&self.previous_signature);
        round_buf.clone_from_slice(&self.round.to_be_bytes());

        let mut hasher = Sha256::new();
        hasher.update(buf);
        Ok(hasher.finalize().to_vec())
    }
}

#[derive(Debug, Serialize, Deserialize)]
/// Unchained drand beacon.
/// Each signature only depends on the round number.
pub struct UnchainedBeacon {
    round: u64,
    #[serde(with = "hex::serde")]
    randomness: Vec<u8>,
    #[serde(with = "hex::serde")]
    signature: Vec<u8>,
}

impl UnchainedBeacon {
    pub fn new(round: u64, randomness: Vec<u8>, signature: Vec<u8>) -> Self {
        Self {
            round,
            randomness,
            signature,
        }
    }
}

impl Message for UnchainedBeacon {
    fn message(&self) -> Result<Vec<u8>> {
        let buf = self.round.to_be_bytes();

        let mut hasher = Sha256::new();
        hasher.update(buf);
        Ok(hasher.finalize().to_vec())
    }
}

#[cfg(test)]
pub mod tests {
    use crate::chain::{
        tests::chained_chain_info,
        tests::{unchained_chain_info, unchained_chain_on_g1_info},
    };

    use super::*;

    /// drand mainnet (curl -sS https://drand.cloudflare.com/public/1000000)
    pub fn chained_beacon() -> RandomnessBeacon {
        serde_json::from_str(r#"{
            "round": 1000000,
            "randomness": "a26ba4d229c666f52a06f1a9be1278dcc7a80dbc1dd2004a1ae7b63cb79fd37e",
            "signature": "87e355169c4410a8ad6d3e7f5094b2122932c1062f603e6628aba2e4cb54f46c3bf1083c3537cd3b99e8296784f46fb40e090961cf9634f02c7dc2a96b69fc3c03735bc419962780a71245b72f81882cf6bb9c961bcf32da5624993bb747c9e5",
            "previous_signature": "86bbc40c9d9347568967add4ddf6e351aff604352a7e1eec9b20dea4ca531ed6c7d38de9956ffc3bb5a7fabe28b3a36b069c8113bd9824135c3bff9b03359476f6b03beec179d4aeff456f4d34bbf702b9af78c3bb44e1892ace8e581bf4afa9"
        }"#).unwrap()
    }

    /// drand testnet (curl -sS https://pl-us.testnet.drand.sh/7672797f548f3f4748ac4bf3352fc6c6b6468c9ad40ad456a397545c6e2df5bf/public/1000000)
    pub fn unchained_beacon() -> RandomnessBeacon {
        serde_json::from_str(r#"{
            "round": 1000000,
            "randomness": "6671747f7d838f18159c474579ea19e8d863e8c25e5271fd7f18ca2ac85181cf",
            "signature": "86b265e10e060805d20dca88f70f6b5e62d5956e7790d32029dfb73fbcd1996bc7aebdea7aeaf74dac0ca2b3ce8f7a6a0399f224a05fe740c0bac9da638212082b0ed21b1a8c5e44a33123f28955ef0713e93e21f6af0cda4073d9a73387434d"
        }"#).unwrap()
    }

    /// drand testnet (curl -sS https://testnet0-api.drand.cloudflare.com/f3827d772c155f95a9fda8901ddd59591a082df5ac6efe3a479ddb1f5eeb202c/public/784604)
    pub fn unchained_beacon_on_g1() -> RandomnessBeacon {
        serde_json::from_str(r#"{
            "round": 784604,
            "randomness": "faf43e19bf00738f5cd8e1904a274f6d9b2184025136660a3e367ea0459f41af",
            "signature": "a029a26d9d0d8e31ef037a81ba5f13a22758b2980e67920c3d233b18b292cd27e106bafd9922a6b1ae69818cd300139f"
        }"#).unwrap()
    }

    /// invalid beacon. Round should be 1,000,000, but it 1
    pub fn invalid_beacon() -> RandomnessBeacon {
        serde_json::from_str(r#"{
            "round": 1,
            "randomness": "a26ba4d229c666f52a06f1a9be1278dcc7a80dbc1dd2004a1ae7b63cb79fd37e",
            "signature": "87e355169c4410a8ad6d3e7f5094b2122932c1062f603e6628aba2e4cb54f46c3bf1083c3537cd3b99e8296784f46fb40e090961cf9634f02c7dc2a96b69fc3c03735bc419962780a71245b72f81882cf6bb9c961bcf32da5624993bb747c9e5",
            "previous_signature": "86bbc40c9d9347568967add4ddf6e351aff604352a7e1eec9b20dea4ca531ed6c7d38de9956ffc3bb5a7fabe28b3a36b069c8113bd9824135c3bff9b03359476f6b03beec179d4aeff456f4d34bbf702b9af78c3bb44e1892ace8e581bf4afa9"
        }"#).unwrap()
    }

    impl PartialEq for RandomnessBeacon {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::ChainedBeacon(chained), Self::ChainedBeacon(other)) => chained == other,
                (Self::UnchainedBeacon(unchained), Self::UnchainedBeacon(other)) => {
                    unchained == other
                }
                _ => false,
            }
        }
    }

    impl PartialEq for ChainedBeacon {
        fn eq(&self, other: &Self) -> bool {
            self.randomness == other.randomness
                && self.round == other.round
                && self.signature == other.signature
                && self.previous_signature == other.previous_signature
        }
    }

    impl PartialEq for UnchainedBeacon {
        fn eq(&self, other: &Self) -> bool {
            self.randomness == other.randomness
                && self.round == other.round
                && self.signature == other.signature
        }
    }

    #[test]
    fn randomness_beacon_verification_success_works() {
        match chained_beacon().verify(chained_chain_info()) {
            Ok(ok) => assert!(ok),
            Err(_err) => panic!("Chained beacon should validate on chained info"),
        }

        match unchained_beacon().verify(unchained_chain_info()) {
            Ok(ok) => assert!(ok),
            Err(_err) => panic!("Unchained beacon should validate on unchained info"),
        }

        match unchained_beacon_on_g1().verify(unchained_chain_on_g1_info()) {
            Ok(ok) => assert!(ok),
            Err(_err) => panic!("Unchained beacon on G1 should validate on unchained info"),
        }
    }

    #[test]
    fn randomness_beacon_verification_failure_works() {
        match invalid_beacon().verify(chained_chain_info()) {
            Ok(ok) => assert!(!ok, "Invalid beacon should not validate"),
            Err(_err) => panic!("Invalid beacon should not validate without returning an error"),
        }

        match unchained_beacon().verify(chained_chain_info()) {
            Ok(ok) => assert!(!ok, "Unchained beacon should not validate on chained info"),
            Err(_err) => panic!(
                "Unchained beacon should not validate on chained info without returning an error"
            ),
        }

        match unchained_beacon_on_g1().verify(unchained_chain_info()) {
            Ok(ok) => assert!(!ok, "Unchained beacon on G1 should not validate on chained info"),
            Err(_err) => panic!(
                "Unchained beacon on G1 should not validate on chained info without returning an error"
            ),
        }
    }
}
