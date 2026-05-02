use std::str;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

// A continuous vector of character bytes, resembling strings with dedublication,
// mirroring the original implementation. Used in the protein graph struct.

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringRef {
    pub start: u32,
    pub len: u32,
}

pub struct StringTable {
    pub buffer: Vec<u8>,
    pub mapping: Vec<StringRef>,
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
}

impl StringTable {
    pub fn build_from_strings<I>(items: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut buffer = Vec::new();
        let mut mapping = Vec::new();

        // Dedup map
        let mut map: HashMap<Box<[u8]>, StringRef> = HashMap::new();

        let iter = items.into_iter();
        let (lower, _) = iter.size_hint();
        mapping.reserve(lower);

        for s in iter {
            let bytes = s.into_bytes();

            match map.entry(bytes.clone().into_boxed_slice()) {
                Entry::Occupied(e) => {
                    mapping.push(*e.get());
                }
                Entry::Vacant(e) => {
                    let start = buffer.len() as u32;
                    let len = bytes.len() as u32;

                    buffer.extend_from_slice(&bytes);

                    // Debug-only UTF-8 validation (moved out of hot path)
                    debug_assert!(
                        std::str::from_utf8(&buffer[start as usize..(start + len) as usize]).is_ok()
                    );

                    let r = StringRef { start, len };
                    e.insert(r);
                    mapping.push(r);
                }
            }
        }

        Self { buffer, mapping }
    }
}