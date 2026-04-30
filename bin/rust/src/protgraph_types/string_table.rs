use std::collections::HashMap;
use std::str;

// A continuous vector of character bytes, resembling strings with dedublication,
// mirroring the original implementation. Used in the protein graph struct.

#[derive(Debug, Clone, Copy)]
pub struct StringRef {
    pub start: u32,
    pub len: u32,
}

#[derive(Debug)]
pub struct StringTable {
    pub buffer: Vec<u8>,
    pub mapping: Vec<StringRef>
}

impl StringTable {
    pub fn build_from_strings<I>(items: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut buffer = Vec::new();
        let mut mapping = Vec::new();
        let mut map: std::collections::HashMap<Vec<u8>, StringRef> = HashMap::new();

        let items: Vec<String> = items.into_iter().collect();
        let count = items.len();
        mapping.resize(count, StringRef { start: 0, len: 0 });

        for (i, s) in items.into_iter().enumerate() {
            let bytes = s.into_bytes();

            if let Some(&r) = map.get(&bytes) {
                mapping[i] = r;
                continue;
            }

            let start = buffer.len() as u32;
            let len = bytes.len() as u32;

            buffer.extend_from_slice(&bytes);

            let r = StringRef { start, len };

            map.insert(bytes, r);
            mapping[i] = r;
        }

        Self { buffer, mapping }
    }

    pub fn get_str(&self, idx: usize) -> anyhow::Result<&str> {
        let r = &self.mapping[idx];
        let start = r.start as usize;
        let len = r.len as usize;
        let bytes = &self.buffer[start..start + len];
        str::from_utf8(bytes).map_err(|e| anyhow::anyhow!("Invalid UTF-8 in StringTable at index {}: {}", idx, e))
    }
}
