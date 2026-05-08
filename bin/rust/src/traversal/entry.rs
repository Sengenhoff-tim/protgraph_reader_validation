use bincode::{Encode, Decode};

#[derive(Encode, Decode, Debug)]
pub struct Entry {
    pub pep_hash: u64,
    pub pep: String,
    pub acc: String,
    pub qualifiers: String,
    pub spos: Option<u16>,
    pub epos: Option<u16>,
    pub mssclvg: u32,
}