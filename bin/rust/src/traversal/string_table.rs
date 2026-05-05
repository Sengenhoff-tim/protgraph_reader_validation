use std::str;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use xxhash_rust::xxh3::xxh3_64;
use serde::{Serialize, Deserialize};

// A continuous vector of character bytes, resembling strings with dedublication,
// mirroring the original implementation. Used in the protein graph struct.

#[repr(C)]
#[derive(Clone, Copy)]

#[derive(Serialize, Deserialize)]
pub struct StringRef {
    pub start: u32,
    pub len: u32,
}


#[derive(Serialize, Deserialize)]
pub struct StringTable {
    pub buffer: Vec<u8>,
    pub mapping: Vec<StringRef>,
    pub hashes: Option<Vec<u64>>,
}

impl StringTable {
    #[inline(always)]
    pub fn get_str(&self, idx: usize) -> &str {
        let r = unsafe { *self.mapping.get_unchecked(idx) };

        unsafe {
            std::str::from_utf8_unchecked(
                self.buffer.get_unchecked(
                    r.start as usize .. r.start as usize + r.len as usize
                )
            )
        }
    }

    #[inline(always)]
    pub fn get_bytes(&self, idx: usize) -> &[u8] {
        let r = unsafe { *self.mapping.get_unchecked(idx) };

        unsafe {
            self.buffer.get_unchecked(
                r.start as usize .. r.start as usize + r.len as usize
            )
        }
    }

    #[inline(always)]
    pub fn get_hash(&self, idx: usize) -> Option<u64> {
        self.hashes
            .as_ref()
            .map(|h| unsafe { *h.get_unchecked(idx) })
    }

pub fn build_from_strings<I>(items: I, with_hashes: bool) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut buffer = Vec::new();
        let mut mapping = Vec::new();

        let mut hashes = if with_hashes {
            Some(Vec::new())
        } else {
            None
        };

        // Dedup map: store also index into mapping
        let mut map: HashMap<Box<[u8]>, (StringRef, u32)> = HashMap::new();

        let iter = items.into_iter();
        let (lower, _) = iter.size_hint();
        mapping.reserve(lower);
        if let Some(h) = hashes.as_mut() {
            h.reserve(lower);
        }

        for s in iter {
            let bytes = s.into_bytes();

            match map.entry(bytes.clone().into_boxed_slice()) {
                Entry::Occupied(e) => {
                    let (r, idx) = *e.get();
                    mapping.push(r);

                    if let Some(h) = hashes.as_mut() {
                        h.push(h[idx as usize]); // reuse existing hash
                    }
                }
                Entry::Vacant(e) => {
                    let start = buffer.len() as u32;
                    let len = bytes.len() as u32;

                    buffer.extend_from_slice(&bytes);

                    let r = StringRef { start, len };
                    let idx = mapping.len() as u32;

                    // compute hash only once per unique string
                    if let Some(h) = hashes.as_mut() {
                        let slice = &buffer[start as usize..(start + len) as usize];
                        let hash = xxh3_64(slice);
                        h.push(hash);
                    }

                    e.insert((r, idx));
                    mapping.push(r);
                }
            }
        }

        Self {
            buffer,
            mapping,
            hashes,
        }
    }
}