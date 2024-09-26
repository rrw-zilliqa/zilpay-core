use crate::{
    address::Address,
    zq1_proto::{Code, Data, Nonce, ProtoTransactionCoreInfo},
};
use ethers::types::H160;
use ethers::utils::to_checksum;
use std::{
    fmt::{Display, Formatter},
    ops::Sub,
    str::FromStr,
};
// use crypto::schnorr::PublicKey;
use crate::pubkey::PubKey;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

pub const EVM_GAS_PER_SCILLA_GAS: u64 = 420;

impl ScillaGas {
    pub fn from_raw(v: u64) -> Self {
        Self(v)
    }

    pub fn checked_sub(self, rhs: ScillaGas) -> Option<ScillaGas> {
        Some(ScillaGas(self.0.checked_sub(rhs.0)?))
    }
}

impl Sub for ScillaGas {
    type Output = ScillaGas;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).expect("scilla gas underflow")
    }
}

// impl From<EvmGas> for ScillaGas {
//     fn from(gas: EvmGas) -> Self {
//         ScillaGas(gas.0 / EVM_GAS_PER_SCILLA_GAS)
//     }
// }

impl Display for ScillaGas {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for ScillaGas {
    type Err = <u64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(u64::from_str(s)?))
    }
}

/// A quantity of Scilla gas. This is the currency used to pay for [TxZilliqa] transactions. When EVM gas is converted
/// to Scilla gas, the quantity is rounded down.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScillaGas(pub u64);

/// A wrapper for ZIL amounts in the Zilliqa API. These are represented in units of (10^-12) ZILs, rather than (10^-18)
/// like in the rest of our code. The implementations of [Serialize], [Deserialize], [Display] and [FromStr] represent
/// the amount in units of (10^-12) ZILs, so this type can be used in the Zilliqa API layer.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ZilAmount(u128);

impl ZilAmount {
    /// Construct a [ZilAmount] from an amount in (10^-18) ZILs. The value will be truncated and rounded down.
    pub fn from_amount(amount: u128) -> ZilAmount {
        ZilAmount(amount / 10u128.pow(6))
    }

    // Construct a [ZilAmount] from an amount in (10^-12) ZILs.
    pub fn from_raw(amount: u128) -> ZilAmount {
        ZilAmount(amount)
    }

    /// Get the ZIL amount in units of (10^-18) ZILs.
    pub fn get(self) -> u128 {
        self.0.checked_mul(10u128.pow(6)).expect("amount overflow")
    }

    /// Return the memory representation of this amount as a big-endian byte array.
    pub fn to_be_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }
}

impl Serialize for ZilAmount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Ok(serializer.serialize_str(&format!("{}", self.0))?)
    }
}

impl<'de> Deserialize<'de> for ZilAmount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        Ok(Self::from_raw(
            u128::from_str(s).map_err(de::Error::custom)?,
        ))
    }
}

impl Serialize for ScillaGas {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Ok(serializer.serialize_str(&format!("{}", self.0))?)
    }
}

impl<'de> Deserialize<'de> for ScillaGas {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        Ok(Self::from_raw(u64::from_str(s).map_err(de::Error::custom)?))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZILTransactionRequest {
    pub version: u32,
    pub nonce: u64,
    #[serde(rename = "gasPrice")]
    pub gas_price: ZilAmount,
    #[serde(rename = "gasLimit")]
    pub gas_limit: ScillaGas,
    #[serde(
        rename = "toAddr",
        serialize_with = "serialize_addr",
        deserialize_with = "deserialize_addr"
    )]
    pub to_addr: Address,
    #[serde(rename = "pubKey")]
    // Should really use serialize_with
    pub pubkey: String,
    pub amount: ZilAmount,
    pub code: String,
    pub data: String,
    pub priority: bool,
    pub signature: String,
}

fn serialize_addr<S>(v: &Address, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // This needs to be a checksummed base-16 address.
    // @todo use zilliqa checksums for zilliqa addresses.
    let summed = H160::from_slice(&v.to_bytes()[1..]);
    let as_string = format!("{}", to_checksum(&summed, None));
    //let bytes = &v.to_bytes()[1..];
    //let as_string = format!("0x{}", &hex::encode(bytes));
    serializer.serialize_str(&as_string)
}

fn deserialize_addr<'de, D>(deserializer: D) -> Result<Address, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    String::deserialize(deserializer).and_then(|my_string| {
        Address::from_zil_base16(
            my_string
                .strip_prefix("0x")
                .unwrap_or_else(|| my_string.as_ref()),
        )
        .map_err(Error::custom)
    })
}

fn version_from_chainid(chain_id: u16) -> u32 {
    ((chain_id as u32) << 16) | 0x0001
}

impl ZILTransactionRequest {
    // Construct an unsigned transaction request.
    #[allow(clippy::too_many_arguments)]
    pub fn from_params(
        chain_id: u16,
        nonce: u64,
        gas_price: ZilAmount,
        gas_limit: ScillaGas,
        to_addr: Address,
        amount: ZilAmount,
        code: Option<String>,
        data: Option<String>,
        priority: bool,
    ) -> Self {
        Self {
            version: version_from_chainid(chain_id),
            nonce,
            gas_price,
            gas_limit,
            to_addr,
            pubkey: "".to_string(),
            amount,
            code: code.unwrap_or_default(),
            data: data.unwrap_or_default(),
            priority,
            signature: "".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZILTransactionReceipt {
    pub chain_id: u16,
    pub nonce: u64,
    pub gas_price: ZilAmount,
    pub gas_limit: ScillaGas,
    pub to_addr: Address,
    pub amount: ZilAmount,
    pub code: String,
    pub data: String,
    pub signature: String,
}

pub fn encode_zilliqa_transaction(txn: &ZILTransactionRequest, pub_key: &PubKey) -> Vec<u8> {
    let oneof8 = (!txn.code.is_empty()).then_some(Code::Code(txn.code.clone().into_bytes()));
    let oneof9 = (!txn.data.is_empty()).then_some(Data::Data(txn.data.clone().into_bytes()));
    let proto = ProtoTransactionCoreInfo {
        version: txn.version,
        toaddr: txn.to_addr.addr_bytes().to_vec(),
        senderpubkey: Some(pub_key.as_ref().to_vec().into()),
        amount: Some((txn.amount).to_be_bytes().to_vec().into()),
        gasprice: Some((txn.gas_price).to_be_bytes().to_vec().into()),
        gaslimit: txn.gas_limit.0,
        oneof2: Some(Nonce::Nonce(txn.nonce)),
        oneof8,
        oneof9,
    };

    prost::Message::encode_to_vec(&proto)
}
