pub(crate) const DATABASE_MAGIC: [u8; 4] = *b"BAG1";
pub(crate) const DATABASE_HEADER_SIZE: usize = 36;

/// Encode a 6-char postal code into a compact sortable integer.
pub fn encode_pc(s: &[u8]) -> u32 {
    let digits = (s[0] - b'0') as u32 * 1000
        + (s[1] - b'0') as u32 * 100
        + (s[2] - b'0') as u32 * 10
        + (s[3] - b'0') as u32;

    let l0 = (s[4] - b'A') as u32; // 0..25
    let l1 = (s[5] - b'A') as u32; // 0..25

    (digits << 18) | (l0 << 13) | (l1 << 8)
}

pub(crate) fn normalize_postalcode(postalcode: &str) -> Option<[u8; 6]> {
    let bytes = postalcode.as_bytes();
    if bytes.len() != 6 {
        return None;
    }

    let mut normalized = [0u8; 6];
    for (dst, src) in normalized.iter_mut().zip(bytes.iter()) {
        *dst = src.to_ascii_uppercase();
    }

    Some(normalized)
}

pub(crate) fn partition_point_range<F>(len: usize, mut pred: F) -> usize
where
    F: FnMut(usize) -> bool,
{
    let mut left = 0usize;
    let mut right = len;
    while left < right {
        let mid = left + (right - left) / 2;
        if pred(mid) {
            left = mid + 1;
        } else {
            right = mid;
        }
    }
    left
}

#[cfg(test)]
mod tests {
    use super::encode_pc;

    #[test]
    fn encode_pc_basic() {
        let encoded = encode_pc(b"1234AB");
        let digits = 1234u32 << 18;
        let letters = 1u32 << 8;
        assert_eq!(encoded, digits | letters);
    }

    #[test]
    fn encode_pc_max_letters() {
        let encoded = encode_pc(b"0000ZZ");
        let letters = (25u32 << 13) | (25u32 << 8);
        assert_eq!(encoded, letters);
    }

    #[test]
    fn encode_pc_mixed() {
        let encoded = encode_pc(b"9876QX");
        let digits = 9876u32 << 18;
        let letters = (16u32 << 13) | (23u32 << 8);
        assert_eq!(encoded, digits | letters);
    }
}
