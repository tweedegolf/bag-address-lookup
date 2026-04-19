use std::collections::HashMap;

pub(crate) const DATABASE_MAGIC: [u8; 4] = *b"BAG4";
pub(crate) const DATABASE_HEADER_SIZE: usize = 84;

pub(crate) struct UniqueFlags {
    pub(crate) locality_unique: Vec<bool>,
    pub(crate) municipality_unique: Vec<bool>,
}

/// Compute per-entity uniqueness across localities and municipalities.
///
/// A locality in municipality M is unique if no other locality outside M
/// and no municipality besides M shares its name. Symmetrically, a
/// municipality is unique if no other municipality shares its name and no
/// locality outside it does either (localities inside it that match its
/// name are treated as the same named place, not a collision).
pub(crate) fn compute_unique_flags(
    locality_names: &[&str],
    municipality_names: &[&str],
    locality_municipality: &[u16],
    locality_had_suffix: &[bool],
    municipality_had_suffix: &[bool],
) -> UniqueFlags {
    let mut count_l: HashMap<&str, u32> = HashMap::new();
    let mut count_m: HashMap<&str, u32> = HashMap::new();
    for name in locality_names {
        *count_l.entry(name).or_insert(0) += 1;
    }
    for name in municipality_names {
        *count_m.entry(name).or_insert(0) += 1;
    }

    // Does each municipality contain at least one locality sharing its name?
    let mut has_self = vec![false; municipality_names.len()];
    for (i, name) in locality_names.iter().enumerate() {
        let m_idx = locality_municipality.get(i).copied().unwrap_or(u16::MAX);
        if m_idx == u16::MAX || (m_idx as usize) >= municipality_names.len() {
            continue;
        }
        if municipality_names[m_idx as usize] == *name {
            has_self[m_idx as usize] = true;
        }
    }

    // Names that carried a disambiguating province suffix in the source data
    // (BAG for localities, CBS for municipalities) are always marked as
    // non-unique — the source registrars kept the suffix because the name has
    // historical ambiguity, even when no live duplicate remains today.
    let locality_unique: Vec<bool> = locality_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if locality_had_suffix.get(i).copied().unwrap_or(false) {
                return false;
            }
            let m_idx = locality_municipality.get(i).copied().unwrap_or(u16::MAX);
            let parent_matches = (m_idx as usize) < municipality_names.len()
                && m_idx != u16::MAX
                && municipality_names[m_idx as usize] == *name;
            let other_localities = count_l.get(name).copied().unwrap_or(0).saturating_sub(1);
            let mut other_munis = count_m.get(name).copied().unwrap_or(0);
            if parent_matches {
                other_munis = other_munis.saturating_sub(1);
            }
            other_localities + other_munis == 0
        })
        .collect();

    let municipality_unique: Vec<bool> = municipality_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if municipality_had_suffix.get(i).copied().unwrap_or(false) {
                return false;
            }
            let mut other_localities = count_l.get(name).copied().unwrap_or(0);
            if has_self[i] {
                other_localities = other_localities.saturating_sub(1);
            }
            let other_munis = count_m.get(name).copied().unwrap_or(0).saturating_sub(1);
            other_localities + other_munis == 0
        })
        .collect();

    UniqueFlags {
        locality_unique,
        municipality_unique,
    }
}

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
