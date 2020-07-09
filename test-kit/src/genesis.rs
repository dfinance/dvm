use libra::prelude::*;
use serde_derive::Deserialize;

pub fn genesis_write_set() -> WriteSet {
    let genesis = include_str!("../resources/genesis.json");
    let ws = serde_json::from_str::<Vec<Row>>(genesis)
        .unwrap()
        .into_iter()
        .map(|row| row.into())
        .collect();
    WriteSetMut::new(ws).freeze().unwrap()
}

#[derive(Debug, Deserialize)]
struct Row {
    address: String,
    path: String,
    value: String,
}

impl Into<(AccessPath, WriteOp)> for Row {
    fn into(self) -> (AccessPath, WriteOp) {
        let address = AccountAddress::from_hex_literal(&format!("0x{}", self.address)).unwrap();
        let path = hex::decode(self.path).unwrap();
        (
            AccessPath::new(address, path),
            WriteOp::Value(hex::decode(self.value).unwrap()),
        )
    }
}
