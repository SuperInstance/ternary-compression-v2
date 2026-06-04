#![forbid(unsafe_code)]

//! Advanced ternary compression for streams of {-1, 0, +1} values.
//!
//! Provides run-length encoding, ternary Huffman coding, LZW adapted for
//! the ternary alphabet, dictionary compression, entropy coding, and
//! compression ratio tracking.

use std::collections::HashMap;

/// A ternary trit value: Negative(-1), Zero(0), or Positive(+1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trit {
    Neg,
    Zero,
    Pos,
}

impl Trit {
    pub fn to_i8(self) -> i8 {
        match self {
            Trit::Neg => -1,
            Trit::Zero => 0,
            Trit::Pos => 1,
        }
    }

    pub fn from_i8(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Trit::Neg),
            0 => Some(Trit::Zero),
            1 => Some(Trit::Pos),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Run-Length Encoding
// ---------------------------------------------------------------------------

/// Run-length encoded entry: a trit and its repeat count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RleEntry {
    pub trit: Trit,
    pub count: usize,
}

/// Compress a ternary stream using run-length encoding.
pub fn rle_compress(data: &[Trit]) -> Vec<RleEntry> {
    if data.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut current = data[0];
    let mut count = 1usize;
    for &t in &data[1..] {
        if t == current {
            count += 1;
        } else {
            result.push(RleEntry { trit: current, count });
            current = t;
            count = 1;
        }
    }
    result.push(RleEntry { trit: current, count });
    result
}

/// Decompress RLE back to a ternary stream.
pub fn rle_decompress(entries: &[RleEntry]) -> Vec<Trit> {
    let mut result = Vec::new();
    for e in entries {
        for _ in 0..e.count {
            result.push(e.trit);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Ternary Huffman Coding
// ---------------------------------------------------------------------------

/// A node in the Huffman tree.
#[derive(Debug, Clone)]
pub enum HuffNode {
    Leaf { trit: Trit, weight: usize },
    Internal { left: Box<HuffNode>, right: Box<HuffNode>, weight: usize },
}

impl HuffNode {
    pub fn weight(&self) -> usize {
        match self {
            HuffNode::Leaf { weight, .. } => *weight,
            HuffNode::Internal { weight, .. } => *weight,
        }
    }

    /// Build code table: maps each Trit to a bit string (Vec<u8> of 0/1).
    pub fn build_codes(&self) -> HashMap<Trit, Vec<u8>> {
        let mut codes = HashMap::new();
        self.build_codes_inner(&mut Vec::new(), &mut codes);
        codes
    }

    fn build_codes_inner(&self, prefix: &mut Vec<u8>, codes: &mut HashMap<Trit, Vec<u8>>) {
        match self {
            HuffNode::Leaf { trit, .. } => {
                codes.insert(*trit, prefix.clone());
            }
            HuffNode::Internal { left, right, .. } => {
                prefix.push(0);
                left.build_codes_inner(prefix, codes);
                prefix.pop();
                prefix.push(1);
                right.build_codes_inner(prefix, codes);
                prefix.pop();
            }
        }
    }
}

/// Build a Huffman tree from frequency counts of ternary symbols.
pub fn build_huffman_tree(freqs: &HashMap<Trit, usize>) -> Option<HuffNode> {
    if freqs.is_empty() {
        return None;
    }
    let mut nodes: Vec<HuffNode> = freqs
        .iter()
        .filter(|(_, &w)| w > 0)
        .map(|(&t, &w)| HuffNode::Leaf { trit: t, weight: w })
        .collect();

    if nodes.is_empty() {
        return None;
    }
    if nodes.len() == 1 {
        return Some(nodes.into_iter().next().unwrap());
    }

    while nodes.len() > 1 {
        nodes.sort_by_key(|n| n.weight());
        let a = nodes.remove(0);
        let b = nodes.remove(0);
        let w = a.weight() + b.weight();
        nodes.push(HuffNode::Internal {
            left: Box::new(a),
            right: Box::new(b),
            weight: w,
        });
    }
    Some(nodes.into_iter().next().unwrap())
}

/// Encode a ternary stream using Huffman codes. Returns a Vec of bits (0/1).
pub fn huffman_encode(data: &[Trit], codes: &HashMap<Trit, Vec<u8>>) -> Vec<u8> {
    let mut bits = Vec::new();
    for &t in data {
        if let Some(code) = codes.get(&t) {
            bits.extend_from_slice(code);
        }
    }
    bits
}

/// Decode a bit stream using a Huffman tree. Returns the decoded ternary stream.
pub fn huffman_decode(bits: &[u8], tree: &HuffNode) -> Vec<Trit> {
    let mut result = Vec::new();
    let mut node = tree;
    for &bit in bits {
        node = match node {
            HuffNode::Leaf { .. } => {
                // restart from root for next symbol
                result.push(match node {
                    HuffNode::Leaf { trit, .. } => *trit,
                    _ => unreachable!(),
                });
                node = tree;
                match node {
                    HuffNode::Internal { left, right, .. } => {
                        if bit == 0 { left.as_ref() } else { right.as_ref() }
                    }
                    _ => node,
                }
            }
            HuffNode::Internal { left, right, .. } => {
                if bit == 0 { left.as_ref() } else { right.as_ref() }
            }
        };
        if let HuffNode::Leaf { trit, .. } = node {
            result.push(*trit);
            node = tree;
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Ternary LZW
// ---------------------------------------------------------------------------

/// LZW dictionary entry: a sequence of trits mapped to a code.
type LzwCode = u16;

/// Compress a ternary stream using LZW adapted for the {-1,0,+1} alphabet.
pub fn lzw_compress(data: &[Trit]) -> Vec<LzwCode> {
    if data.is_empty() {
        return Vec::new();
    }
    // Initial dictionary: single trits → codes 0,1,2
    let mut dict: HashMap<Vec<Trit>, LzwCode> = HashMap::new();
    dict.insert(vec![Trit::Neg], 0);
    dict.insert(vec![Trit::Zero], 1);
    dict.insert(vec![Trit::Pos], 2);
    let mut next_code: LzwCode = 3;

    let mut output = Vec::new();
    let mut w = Vec::new();

    for &t in data {
        let mut wc = w.clone();
        wc.push(t);
        if dict.contains_key(&wc) {
            w = wc;
        } else {
            output.push(dict[&w]);
            if next_code < 4096 {
                dict.insert(wc, next_code);
                next_code += 1;
            }
            w = vec![t];
        }
    }
    if !w.is_empty() {
        output.push(dict[&w]);
    }
    output
}

/// Decompress LZW codes back to a ternary stream.
pub fn lzw_decompress(codes: &[LzwCode]) -> Vec<Trit> {
    if codes.is_empty() {
        return Vec::new();
    }
    let mut dict: HashMap<LzwCode, Vec<Trit>> = HashMap::new();
    dict.insert(0, vec![Trit::Neg]);
    dict.insert(1, vec![Trit::Zero]);
    dict.insert(2, vec![Trit::Pos]);
    let mut next_code: LzwCode = 3;

    let mut result = Vec::new();
    let mut prev = dict[&codes[0]].clone();
    result.extend_from_slice(&prev);

    for &code in &codes[1..] {
        let entry = if let Some(e) = dict.get(&code) {
            e.clone()
        } else if code == next_code && !prev.is_empty() {
            let mut e = prev.clone();
            e.push(prev[0]);
            e
        } else {
            Vec::new()
        };

        result.extend_from_slice(&entry);
        if next_code < 4096 && !prev.is_empty() {
            let mut new_entry = prev.clone();
            new_entry.push(entry[0]);
            dict.insert(next_code, new_entry);
            next_code += 1;
        }
        prev = entry;
    }
    result
}

// ---------------------------------------------------------------------------
// Dictionary Compression
// ---------------------------------------------------------------------------

/// A simple dictionary-based compressor that finds repeated ternary patterns.
pub struct DictionaryCompressor {
    /// Map from pattern to replacement code.
    dictionary: HashMap<Vec<Trit>, usize>,
}

impl DictionaryCompressor {
    pub fn new() -> Self {
        Self {
            dictionary: HashMap::new(),
        }
    }

    /// Build a dictionary from data by finding frequent n-grams.
    pub fn build_dictionary(&mut self, data: &[Trit], min_len: usize, max_len: usize, min_freq: usize) {
        self.dictionary.clear();
        let n = data.len();
        let mut freq: HashMap<Vec<Trit>, usize> = HashMap::new();
        for len in min_len..=max_len {
            if len > n { continue; }
            for i in 0..=n.saturating_sub(len) {
                if i + len > n { continue; }
                let pattern = &data[i..i + len];
                *freq.entry(pattern.to_vec()).or_insert(0) += 1;
            }
        }
        let mut code = 0usize;
        let mut entries: Vec<_> = freq.iter().filter(|(_, &f)| f >= min_freq).collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        for (pattern, _) in entries {
            self.dictionary.insert(pattern.clone(), code);
            code += 1;
            if code >= 256 {
                break;
            }
        }
    }

    /// Compress data by replacing known patterns with their codes.
    /// Returns a mixed stream: literal trits (as i8) and pattern codes (as i16, negative codes).
    pub fn compress(&self, data: &[Trit]) -> Vec<DictToken> {
        let mut result = Vec::new();
        let mut i = 0;
        let n = data.len();
        while i < n {
            let mut best_len = 0usize;
            let mut best_code = None;
            let max_pat = std::cmp::min(16, n - i);
            for len in (2..=max_pat).rev() {
                let window = &data[i..i + len];
                if let Some(&code) = self.dictionary.get(window) {
                    best_len = len;
                    best_code = Some(code);
                    break;
                }
            }
            if best_len > 0 {
                result.push(DictToken::Pattern { code: best_code.unwrap(), len: best_len });
                i += best_len;
            } else {
                result.push(DictToken::Literal(data[i]));
                i += 1;
            }
        }
        result
    }

    /// Decompress tokens using the reverse dictionary.
    pub fn decompress(&self, tokens: &[DictToken]) -> Vec<Trit> {
        let mut rev: HashMap<usize, Vec<Trit>> = HashMap::new();
        for (pat, &code) in &self.dictionary {
            rev.insert(code, pat.clone());
        }
        let mut result = Vec::new();
        for token in tokens {
            match token {
                DictToken::Literal(t) => result.push(*t),
                DictToken::Pattern { code, .. } => {
                    if let Some(pat) = rev.get(code) {
                        result.extend_from_slice(pat);
                    }
                }
            }
        }
        result
    }

    pub fn dictionary_size(&self) -> usize {
        self.dictionary.len()
    }
}

/// A token in dictionary-compressed output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictToken {
    Literal(Trit),
    Pattern { code: usize, len: usize },
}

// ---------------------------------------------------------------------------
// Entropy Coding
// ---------------------------------------------------------------------------

/// Calculate Shannon entropy of a ternary stream (in bits per symbol).
pub fn shannon_entropy(data: &[Trit]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let n = data.len() as f64;
    let mut counts: HashMap<Trit, usize> = HashMap::new();
    for &t in data {
        *counts.entry(t).or_insert(0) += 1;
    }
    let mut entropy = 0.0;
    for &count in counts.values() {
        if count > 0 {
            let p = count as f64 / n;
            entropy -= p * p.log2();
        }
    }
    entropy
}

/// Encode ternary data using arithmetic-like entropy coding (simplified).
/// Returns bytes where each byte packs 4 trits (2 bits each, values 0-2 mapped from Neg/Zero/Pos).
pub fn entropy_encode(data: &[Trit]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut byte = 0u8;
    let mut shift = 6;
    for &t in data {
        let val: u8 = match t {
            Trit::Neg => 0,
            Trit::Zero => 1,
            Trit::Pos => 2,
        };
        byte |= val << shift;
        if shift == 0 {
            result.push(byte);
            byte = 0;
            shift = 6;
        } else {
            shift -= 2;
        }
    }
    if shift < 6 {
        result.push(byte);
    }
    result
}

/// Decode entropy-encoded bytes back to ternary stream.
pub fn entropy_decode(bytes: &[u8], original_len: usize) -> Vec<Trit> {
    let mut result = Vec::new();
    for &byte in bytes {
        for shift in (0..8).step_by(2).rev() {
            if result.len() >= original_len {
                break;
            }
            let val = (byte >> shift) & 0b11;
            result.push(match val {
                0 => Trit::Neg,
                1 => Trit::Zero,
                2 => Trit::Pos,
                _ => Trit::Zero, // padding
            });
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Compression Ratio Tracking
// ---------------------------------------------------------------------------

/// Tracks compression ratios across multiple runs.
#[derive(Debug, Clone)]
pub struct CompressionTracker {
    pub original_size: usize,
    pub compressed_sizes: Vec<usize>,
    pub method_names: Vec<String>,
}

impl CompressionTracker {
    pub fn new(original_size: usize) -> Self {
        Self {
            original_size,
            compressed_sizes: Vec::new(),
            method_names: Vec::new(),
        }
    }

    pub fn record(&mut self, method: &str, compressed_size: usize) {
        self.method_names.push(method.to_string());
        self.compressed_sizes.push(compressed_size);
    }

    /// Calculate ratio for a specific recording (lower is better; <1.0 means compression).
    pub fn ratio(&self, index: usize) -> f64 {
        if index >= self.compressed_sizes.len() || self.original_size == 0 {
            return 1.0;
        }
        self.compressed_sizes[index] as f64 / self.original_size as f64
    }

    /// Find the best (lowest ratio) method recorded.
    pub fn best_method(&self) -> Option<(&str, f64)> {
        let mut best_idx = 0usize;
        let mut best_ratio = f64::MAX;
        for (i, &sz) in self.compressed_sizes.iter().enumerate() {
            let r = sz as f64 / self.original_size as f64;
            if r < best_ratio {
                best_ratio = r;
                best_idx = i;
            }
        }
        if self.compressed_sizes.is_empty() {
            return None;
        }
        Some((&self.method_names[best_idx], best_ratio))
    }

    pub fn space_savings_pct(&self, index: usize) -> f64 {
        (1.0 - self.ratio(index)) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> Vec<Trit> {
        vec![
            Trit::Pos, Trit::Pos, Trit::Pos, Trit::Pos, Trit::Pos,
            Trit::Zero, Trit::Zero, Trit::Zero,
            Trit::Neg, Trit::Neg,
            Trit::Pos, Trit::Pos, Trit::Pos,
            Trit::Zero, Trit::Zero, Trit::Zero, Trit::Zero,
            Trit::Neg, Trit::Neg, Trit::Neg, Trit::Neg, Trit::Neg,
        ]
    }

    #[test]
    fn test_trit_conversions() {
        assert_eq!(Trit::Neg.to_i8(), -1);
        assert_eq!(Trit::Zero.to_i8(), 0);
        assert_eq!(Trit::Pos.to_i8(), 1);
        assert_eq!(Trit::from_i8(-1), Some(Trit::Neg));
        assert_eq!(Trit::from_i8(0), Some(Trit::Zero));
        assert_eq!(Trit::from_i8(1), Some(Trit::Pos));
        assert_eq!(Trit::from_i8(2), None);
    }

    #[test]
    fn test_rle_empty() {
        let compressed = rle_compress(&[]);
        assert!(compressed.is_empty());
        let decompressed = rle_decompress(&[]);
        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_rle_roundtrip() {
        let data = sample_data();
        let compressed = rle_compress(&data);
        let decompressed = rle_decompress(&compressed);
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_rle_counts() {
        let compressed = rle_compress(&sample_data());
        assert_eq!(compressed.len(), 6); // 6 runs in sample_data
        assert_eq!(compressed[0].count, 5); // Pos x5
        assert_eq!(compressed[1].count, 3); // Zero x3
        assert_eq!(compressed[2].count, 2); // Neg x2
    }

    #[test]
    fn test_rle_single_element() {
        let data = vec![Trit::Pos];
        let compressed = rle_compress(&data);
        assert_eq!(compressed.len(), 1);
        assert_eq!(compressed[0].trit, Trit::Pos);
        assert_eq!(compressed[0].count, 1);
    }

    #[test]
    fn test_rle_all_same() {
        let data = vec![Trit::Zero; 100];
        let compressed = rle_compress(&data);
        assert_eq!(compressed.len(), 1);
        assert_eq!(compressed[0].count, 100);
    }

    #[test]
    fn test_huffman_build_tree() {
        let mut freqs = HashMap::new();
        freqs.insert(Trit::Pos, 10);
        freqs.insert(Trit::Zero, 5);
        freqs.insert(Trit::Neg, 2);
        let tree = build_huffman_tree(&freqs).unwrap();
        let codes = tree.build_codes();
        // Most frequent should have shortest code
        assert!(codes[&Trit::Pos].len() <= codes[&Trit::Neg].len());
    }

    #[test]
    fn test_huffman_encode_decode() {
        let data = sample_data();
        let mut freqs = HashMap::new();
        for &t in &data {
            *freqs.entry(t).or_insert(0) += 1;
        }
        let tree = build_huffman_tree(&freqs).unwrap();
        let codes = tree.build_codes();
        let encoded = huffman_encode(&data, &codes);
        let decoded = huffman_decode(&encoded, &tree);
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_huffman_empty() {
        let freqs = HashMap::new();
        assert!(build_huffman_tree(&freqs).is_none());
    }

    #[test]
    fn test_huffman_single_symbol() {
        let mut freqs = HashMap::new();
        freqs.insert(Trit::Pos, 5);
        let tree = build_huffman_tree(&freqs).unwrap();
        let codes = tree.build_codes();
        assert!(codes[&Trit::Pos].is_empty()); // single symbol has empty code
    }

    #[test]
    fn test_lzw_roundtrip() {
        let data = sample_data();
        let compressed = lzw_compress(&data);
        let decompressed = lzw_decompress(&compressed);
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_lzw_empty() {
        assert!(lzw_compress(&[]).is_empty());
        assert!(lzw_decompress(&[]).is_empty());
    }

    #[test]
    fn test_lzw_simple() {
        let data = vec![Trit::Pos, Trit::Pos, Trit::Pos, Trit::Pos, Trit::Pos];
        let compressed = lzw_compress(&data);
        assert!(!compressed.is_empty());
        let decompressed = lzw_decompress(&compressed);
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_lzw_repeating_pattern() {
        let mut data = Vec::new();
        for _ in 0..20 {
            data.push(Trit::Pos);
            data.push(Trit::Zero);
            data.push(Trit::Neg);
        }
        let compressed = lzw_compress(&data);
        // Should compress significantly with repeating pattern
        assert!(compressed.len() < data.len());
        let decompressed = lzw_decompress(&compressed);
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_dictionary_compressor() {
        let mut comp = DictionaryCompressor::new();
        let mut data = Vec::new();
        for _ in 0..10 {
            data.extend_from_slice(&[Trit::Pos, Trit::Pos, Trit::Neg]);
        }
        comp.build_dictionary(&data, 2, 4, 2);
        assert!(comp.dictionary_size() > 0);
        let tokens = comp.compress(&data);
        let decompressed = comp.decompress(&tokens);
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_dictionary_empty() {
        let mut comp = DictionaryCompressor::new();
        comp.build_dictionary(&[], 2, 4, 2);
        assert_eq!(comp.dictionary_size(), 0);
        // compress empty data
        let tokens = comp.compress(&[]);
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_shannon_entropy_uniform() {
        let data = vec![Trit::Neg, Trit::Zero, Trit::Pos, Trit::Neg, Trit::Zero, Trit::Pos];
        let e = shannon_entropy(&data);
        assert!((e - 1.585).abs() < 0.01); // log2(3) ≈ 1.585
    }

    #[test]
    fn test_shannon_entropy_single() {
        let data = vec![Trit::Pos; 100];
        let e = shannon_entropy(&data);
        assert!(e.abs() < 0.001);
    }

    #[test]
    fn test_shannon_entropy_empty() {
        assert_eq!(shannon_entropy(&[]), 0.0);
    }

    #[test]
    fn test_entropy_encode_decode() {
        let data = sample_data();
        let encoded = entropy_encode(&data);
        let decoded = entropy_decode(&encoded, data.len());
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_entropy_encode_size() {
        let data = vec![Trit::Pos; 8];
        let encoded = entropy_encode(&data);
        // 8 trits → 4 per byte → 2 bytes
        assert_eq!(encoded.len(), 2);
    }

    #[test]
    fn test_compression_tracker() {
        let mut tracker = CompressionTracker::new(100);
        tracker.record("rle", 40);
        tracker.record("huffman", 60);
        tracker.record("lzw", 30);
        assert_eq!(tracker.compressed_sizes.len(), 3);
        assert!((tracker.ratio(0) - 0.4).abs() < 0.001);
        assert!((tracker.ratio(1) - 0.6).abs() < 0.001);
        let (name, ratio) = tracker.best_method().unwrap();
        assert_eq!(name, "lzw");
        assert!((ratio - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_tracker_space_savings() {
        let mut tracker = CompressionTracker::new(200);
        tracker.record("rle", 100);
        assert!((tracker.space_savings_pct(0) - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_tracker_no_recordings() {
        let tracker = CompressionTracker::new(100);
        assert!(tracker.best_method().is_none());
    }

    #[test]
    fn test_tracker_zero_original() {
        let mut tracker = CompressionTracker::new(0);
        tracker.record("x", 0);
        assert!((tracker.ratio(0) - 1.0).abs() < 0.001);
    }
}
