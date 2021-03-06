use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

use cid::{Cid, Codec, Error, Prefix, Version};
use multihash::Sha2_256;

#[test]
fn basic_marshalling() {
    let h = Sha2_256::digest(b"beep boop");

    let cid = Cid::new_v1(Codec::DagProtobuf, h);

    let data = cid.to_bytes();
    let out = Cid::try_from(data.clone()).unwrap();
    assert_eq!(cid, out);

    let out2: Cid = data.try_into().unwrap();
    assert_eq!(cid, out2);

    let s = cid.to_string();
    let out3 = Cid::try_from(&s[..]).unwrap();
    assert_eq!(cid, out3);

    let out4: Cid = (&s[..]).try_into().unwrap();
    assert_eq!(cid, out4);
}

#[test]
fn empty_string() {
    assert_eq!(Cid::try_from(""), Err(Error::InputTooShort));
}

#[test]
fn v0_handling() {
    let old = "QmdfTbBqBPQ7VNxZEYEj14VmRuZBkqFbiwReogJgS1zR1n";
    let cid = Cid::try_from(old).unwrap();

    assert_eq!(cid.version, Version::V0);
    assert_eq!(cid.to_string(), old);
}

#[test]
fn from_str() {
    let cid: Cid = "QmdfTbBqBPQ7VNxZEYEj14VmRuZBkqFbiwReogJgS1zR1n"
        .parse()
        .unwrap();
    assert_eq!(cid.version, Version::V0);

    let bad = "QmdfTbBqBPQ7VNxZEYEj14VmRuZBkqFbiwReogJgS1zIII".parse::<Cid>();
    assert_eq!(bad, Err(Error::ParsingError));
}

#[test]
fn v0_error() {
    let bad = "QmdfTbBqBPQ7VNxZEYEj14VmRuZBkqFbiwReogJgS1zIII";
    assert_eq!(Cid::try_from(bad), Err(Error::ParsingError));
}

#[test]
fn prefix_roundtrip() {
    let data = b"awesome test content";
    let h = Sha2_256::digest(data);

    let cid = Cid::new_v1(Codec::DagProtobuf, h);
    let prefix = cid.prefix();

    let cid2 = Cid::new_from_prefix(&prefix, data);

    assert_eq!(cid, cid2);

    let prefix_bytes = prefix.as_bytes();
    let prefix2 = Prefix::new_from_bytes(&prefix_bytes).unwrap();

    assert_eq!(prefix, prefix2);
}

#[test]
fn from() {
    let the_hash = "QmdfTbBqBPQ7VNxZEYEj14VmRuZBkqFbiwReogJgS1zR1n";

    let cases = vec![
        format!("/ipfs/{:}", &the_hash),
        format!("https://ipfs.io/ipfs/{:}", &the_hash),
        format!("http://localhost:8080/ipfs/{:}", &the_hash),
    ];

    for case in cases {
        let cid = Cid::try_from(case).unwrap();
        assert_eq!(cid.version, Version::V0);
        assert_eq!(cid.to_string(), the_hash);
    }
}

#[test]
fn test_hash() {
    let data: Vec<u8> = vec![1, 2, 3];
    let prefix = Prefix {
        version: Version::V0,
        codec: Codec::DagProtobuf,
        mh_type: multihash::Code::Sha2_256,
        mh_len: 32,
    };
    let mut map = HashMap::new();
    let cid = Cid::new_from_prefix(&prefix, &data);
    map.insert(cid.clone(), data.clone());
    assert_eq!(&data, map.get(&cid).unwrap());
}

#[test]
fn test_base32() {
    let cid = Cid::from_str("bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy").unwrap();
    assert_eq!(cid.version, Version::V1);
    assert_eq!(cid.codec, Codec::Raw);
    assert_eq!(cid.hash, Sha2_256::digest(b"foo"));
}
