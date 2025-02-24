use bech32::{Bech32, Hrp};
use config::address::{ADDR_LEN, HRP};
use sha2::{Digest, Sha256};
use zil_errors::address::AddressError;

pub fn from_zil_base16(addr: &str) -> Option<[u8; ADDR_LEN]> {
    let mb_bytes = hex::decode(addr).ok()?;
    let value = mb_bytes.try_into().ok()?;

    Some(value)
}

pub fn from_zil_pub_key(pub_key: &[u8]) -> Result<[u8; ADDR_LEN], AddressError> {
    let mut hasher = Sha256::new();
    hasher.update(pub_key);
    let hash = hasher.finalize();
    let hash_slice = &hash[12..];
    let value: [u8; ADDR_LEN] = hash_slice.try_into().or(Err(AddressError::InvalidPubKey))?;

    Ok(value)
}

pub fn from_zil_bech32_address(address: &str) -> Result<[u8; ADDR_LEN], AddressError> {
    let (hrp, bytes) = bech32::decode(address).map_err(|_| AddressError::InvalidBech32Len)?;
    let bytes: [u8; ADDR_LEN] = bytes.try_into().or(Err(AddressError::InvalidBech32Len))?;

    if hrp.to_string() != HRP {
        return Err(AddressError::InvalidHRP);
    }

    Ok(bytes)
}

pub fn to_zil_bech32(value: &[u8; ADDR_LEN]) -> Result<String, AddressError> {
    let hrp = Hrp::parse(HRP).map_err(|_| AddressError::InvalidHRP)?;

    bech32::encode::<Bech32>(hrp, value).map_err(|_| AddressError::InvalidBech32Len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bech32_address() {
        let bech32 = "zil1w7f636xqn5vf6n2zrnjmckekw3jkckkpyrd6z8";
        let base16_buff = from_zil_bech32_address(bech32).unwrap();
        let base16 = hex::encode(base16_buff);

        assert_eq!(base16, "7793a8e8c09d189d4d421ce5bc5b3674656c5ac1");

        let base16_buff = from_zil_bech32_address("zi21w7f636xqn5vf6n2zrnjmckekw3jkckkpyrd6z8");

        assert_eq!(base16_buff, Err(AddressError::InvalidBech32Len));
    }

    #[test]
    fn test_to_bech32_address() {
        let bech32 = "zil1w7f636xqn5vf6n2zrnjmckekw3jkckkpyrd6z8";
        let addr = from_zil_base16("7793a8e8c09d189d4d421ce5bc5b3674656c5ac1").unwrap();

        assert_eq!(bech32, to_zil_bech32(&addr).unwrap());
    }

    #[test]
    fn test_addr_from_pubkey() {
        let pubkey =
            hex::decode("03150a7f37063b134cde30070431a69148d60b252f4c7b38de33d813d329a7b7da")
                .unwrap();
        let addr = from_zil_pub_key(&pubkey).unwrap();

        assert_eq!(
            to_zil_bech32(&addr).unwrap(),
            "zil1a0vtxuxamd3kltmyzpqdyxqu25vsss8mp58jtu"
        );
    }
}
