use bincode::{Encode, Decode};

#[derive(Encode, Decode, Debug)]
pub struct Entry {
    pub pep: String,
    pub meta: EntryMeta
}

#[derive(Encode, Decode, Debug)]
pub struct EntryMeta {
    pub acc: String,
    pub qualifiers: String,
    pub spos: Option<u16>,
    pub epos: Option<u16>,
    pub mssclvg: u32,
}