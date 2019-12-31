/// ! # cid
/// !
/// ! Implementation of [cid](https://github.com/ipld/cid) in Rust.
use core::{
    convert::TryFrom,
    fmt,
    hash::{Hash, Hasher},
    str::FromStr,
};
use integer_encoding::{VarIntReader, VarIntWriter};
use multibase::Base;
use multihash::{Code, Multihash, MultihashRef};
use std::io::Cursor;

mod codec;
mod error;
mod version;

pub use codec::Codec;
pub use error::Error;
pub use version::Version;

/// Representation of a CID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cid {
    version: Version,
    codec: Codec,
    hash: Multihash,
}

impl Cid {
    /// Create a new CIDv0.
    pub fn new_v0(hash: Multihash) -> Result<Cid, Error> {
        if hash.code() != Code::Sha2_256 {
            return Err(Error::InvalidCidV0Multihash);
        }
        Ok(Cid {
            version: Version::V0,
            codec: Codec::DagProtobuf,
            hash,
        })
    }

    /// Create a new CIDv1.
    pub fn new_v1(codec: Codec, hash: Multihash) -> Cid {
        Cid {
            version: Version::V1,
            codec,
            hash,
        }
    }

    /// Create a new CID.
    pub fn new(version: Version, codec: Codec, hash: Multihash) -> Result<Cid, Error> {
        match version {
            Version::V0 => {
                if codec != Codec::DagProtobuf {
                    return Err(Error::InvalidCidV0Codec);
                }
                Self::new_v0(hash)
            }
            Version::V1 => Ok(Self::new_v1(codec, hash)),
        }
    }

    /// Returns the cid version.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Returns the cid codec.
    pub fn codec(&self) -> Codec {
        self.codec
    }

    /// Returns the cid multihash.
    pub fn hash(&self) -> MultihashRef {
        self.hash.as_ref()
    }

    fn to_string_v0(&self) -> String {
        let mut string = multibase::encode(Base::Base58btc, &self.hash.as_ref());

        // Drop the first character as v0 does not know
        // about multibase
        string.remove(0);

        string
    }

    fn to_string_v1(&self) -> String {
        multibase::encode(Base::Base58btc, self.to_bytes().as_slice())
    }

    /// Returns the string representation.
    pub fn to_string(&self) -> String {
        match self.version {
            Version::V0 => self.to_string_v0(),
            Version::V1 => self.to_string_v1(),
        }
    }

    fn to_bytes_v0(&self) -> Vec<u8> {
        self.hash.to_bytes()
    }

    fn to_bytes_v1(&self) -> Vec<u8> {
        let mut res = Vec::with_capacity(16);
        res.write_varint(u64::from(self.version)).unwrap();
        res.write_varint(u64::from(self.codec)).unwrap();
        res.extend_from_slice(&self.hash.as_ref());
        res
    }

    /// Returns the bytes representation.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self.version {
            Version::V0 => self.to_bytes_v0(),
            Version::V1 => self.to_bytes_v1(),
        }
    }

    #[cfg(feature = "random")]
    /// Generates a random `Cid` with the passed `Rng`.
    pub fn random_with_rng<R: rand::Rng + ?Sized>(rng: &mut R) -> Self {
        use multihash::MultihashDigest;
        Self::new_v0(multihash::Sha2_256::random(rng)).unwrap()
    }

    #[cfg(feature = "random")]
    /// Generates a random `Cid`.
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        Self::random_with_rng(&mut rng)
    }
}

impl From<&Cid> for Cid {
    fn from(cid: &Cid) -> Self {
        cid.to_owned()
    }
}

impl From<Cid> for Vec<u8> {
    fn from(cid: Cid) -> Self {
        cid.to_bytes()
    }
}

impl From<Cid> for String {
    fn from(cid: Cid) -> Self {
        cid.to_string()
    }
}

impl TryFrom<&[u8]> for Cid {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if Version::is_v0_binary(bytes) {
            // Verify that hash can be decoded, this is very cheap
            let hash = multihash::decode(bytes)?;

            Self::new_v0(hash)
        } else {
            let mut cur = Cursor::new(bytes);
            let raw_version = cur.read_varint()?;
            let raw_codec = cur.read_varint()?;

            let version = Version::from(raw_version)?;
            let codec = Codec::from(raw_codec)?;

            let hash = &bytes[cur.position() as usize..];

            // Verify that hash can be decoded, this is very cheap
            let hash = multihash::decode(hash)?;

            Self::new(version, codec, hash)
        }
    }
}

impl TryFrom<Vec<u8>> for Cid {
    type Error = Error;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(bytes.as_slice())
    }
}

impl TryFrom<&str> for Cid {
    type Error = Error;

    fn try_from(cid_str: &str) -> Result<Self, Self::Error> {
        static IPFS_DELIMETER: &str = "/ipfs/";

        let hash = match cid_str.find(IPFS_DELIMETER) {
            Some(index) => &cid_str[index + IPFS_DELIMETER.len()..],
            _ => cid_str,
        };

        if hash.len() < 2 {
            return Err(Error::InputTooShort);
        }

        let (_, bytes) = if Version::is_v0_str(hash) {
            // TODO: could avoid the roundtrip here and just use underlying
            // base-x base58btc decoder here.
            let hash = Base::Base58btc.code().to_string() + hash;

            multibase::decode(hash)
        } else {
            multibase::decode(hash)
        }?;

        Self::try_from(bytes)
    }
}

impl TryFrom<String> for Cid {
    type Error = Error;

    fn try_from(cid_str: String) -> Result<Self, Self::Error> {
        Self::try_from(cid_str.as_str())
    }
}

impl FromStr for Cid {
    type Err = Error;

    fn from_str(cid_str: &str) -> Result<Self, Self::Err> {
        Cid::try_from(cid_str)
    }
}

impl fmt::Display for Cid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Self::to_string(self))
    }
}

impl Hash for Cid {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut hash_bytes = [0u8; 8];
        let cid_bytes = self.hash().to_bytes();
        hash_bytes.copy_from_slice(&cid_bytes[1..9]);
        state.write_u64(u64::from_ne_bytes(hash_bytes));
    }
}

#[cfg(feature = "graphql")]
juniper::graphql_scalar!(Cid {
    description: "Self-describing content-addressed identifiers for distributed systems"

    resolve(&self) -> juniper::Value {
        juniper::Value::scalar(self.to_string())
    }

    from_input_value(v: &InputValue) -> Option<Cid> {
        v.as_scalar_value::<String>().and_then(|s| s.parse().ok())
    }

    from_str<'a>(value: ScalarToken<'a>) -> juniper::ParseScalarResult<'a> {
        <String as juniper::ParseScalarValue>::from_str(value)
    }
});
