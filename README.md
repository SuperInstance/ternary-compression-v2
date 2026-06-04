# ternary-compression-v2

Multi-algorithm compression for ternary data streams — RLE, Huffman, LZW, dictionary compression, and entropy coding for `{-1, 0, +1}` values.

## Why This Exists

Ternary data appears naturally in quantized neural networks, balanced logic systems, and signal processing. Standard compression algorithms assume binary or byte-oriented data, leaving efficiency on the table when your alphabet is only three symbols. This crate provides a suite of compression methods specifically tuned for the ternary domain.

## Core Concepts

- **Trit** — The fundamental unit: `Neg` (-1), `Zero` (0), or `Pos` (+1)
- **Run-Length Encoding (RLE)** — Exploits consecutive repeats of the same trit
- **Ternary Huffman Coding** — Variable-length codes weighted by symbol frequency
- **Ternary LZW** — Dictionary-based compression that builds up pattern tables on the fly
- **Dictionary Compression** — Finds and replaces frequent n-gram patterns
- **Entropy Coding** — Packs 4 trits per byte (2 bits each) for compact representation
- **CompressionTracker** — Benchmarks multiple methods against the same data

## Quick Start

```toml
# Cargo.toml
[dependencies]
ternary-compression-v2 = "0.1"
```

```rust
use ternary_compression_v2::*;

// Create some ternary data
let data: Vec<Trit> = vec![
    Trit::Pos, Trit::Pos, Trit::Pos, Trit::Pos, Trit::Pos,
    Trit::Zero, Trit::Zero, Trit::Zero,
    Trit::Neg, Trit::Neg,
];

// Run-length encoding
let rle = rle_compress(&data);
let decompressed = rle_decompress(&rle);
assert_eq!(data, decompressed);

// Huffman coding
let mut freqs = std::collections::HashMap::new();
for &t in &data { *freqs.entry(t).or_insert(0) += 1; }
let tree = build_huffman_tree(&freqs).unwrap();
let codes = tree.build_codes();
let encoded = huffman_encode(&data, &codes);
let decoded = huffman_decode(&encoded, &tree);
assert_eq!(data, decoded);

// LZW compression
let lzw_codes = lzw_compress(&data);
let lzw_decoded = lzw_decompress(&lzw_codes);
assert_eq!(data, lzw_decoded);

// Entropy coding (compact bit-packing)
let packed = entropy_encode(&data);
let unpacked = entropy_decode(&packed, data.len());
assert_eq!(data, unpacked);

// Compare methods
let mut tracker = CompressionTracker::new(data.len());
tracker.record("rle", rle.len() * std::mem::size_of::<RleEntry>());
tracker.record("huffman", (encoded.len() + 7) / 8);
tracker.record("lzw", lzw_codes.len() * 2);
if let Some((best, ratio)) = tracker.best_method() {
    println!("Best method: {} (ratio: {:.2})", best, ratio);
}
```

## API Overview

| Function / Type | Description |
|---|---|
| `Trit` | Enum: `Neg`, `Zero`, `Pos` with `to_i8()` / `from_i8()` |
| `rle_compress` / `rle_decompress` | Run-length encoding |
| `build_huffman_tree` / `huffman_encode` / `huffman_decode` | Huffman coding |
| `lzw_compress` / `lzw_decompress` | LZW compression |
| `DictionaryCompressor` | N-gram pattern dictionary |
| `shannon_entropy` | Shannon entropy in bits/symbol |
| `entropy_encode` / `entropy_decode` | Bit-packed encoding |
| `CompressionTracker` | Benchmark and compare methods |

## How It Works

**RLE** scans the stream, grouping consecutive identical trits into `(Trit, count)` pairs. Best for data with long runs.

**Huffman** builds a frequency-weighted binary tree, assigning shorter codes to more common trits. Most effective when symbol frequencies are skewed.

**LZW** starts with a 3-entry dictionary (one per trit) and progressively adds multi-trit patterns as it encounters them, replacing repeated patterns with dictionary codes.

**Dictionary Compression** pre-scans data for frequent n-grams, then replaces them with single codes during compression.

**Entropy Coding** packs each trit into 2 bits (4 trits per byte), achieving the information-theoretic baseline.

## Use Cases

1. **Quantized neural network weight storage** — Compress ternary-weighted models (e.g., Trained Ternary Quantization) for deployment on edge devices
2. **Sensor data compression** — Ternary-encoded signals from thresholded sensors can be compressed before transmission
3. **Simulation output** — Large-scale ternary cellular automata or agent-based models produce massive trit streams that compress well with LZW
4. **Communication protocols** — Pack ternary commands efficiently for bandwidth-constrained channels

## Known Limitations

- **LZW dictionary grows unbounded**: `lzw_compress()` adds new patterns to the dictionary on every input position without a maximum size. For long inputs (millions of trits), the dictionary code space grows without bound, negating compression gains. Production LZW implementations cap the dictionary (e.g., 4096 entries) and reset when full.

- **Huffman coding with only 3 symbols provides limited compression**: With a 3-symbol alphabet, the Huffman tree has at most 2 levels, giving codes of length 1 and 2. The theoretical minimum is log₂(3) ≈ 1.585 bits/symbol, and Huffman achieves at most ~1.67 bits/symbol — only a 16% improvement over raw 2-bit packing. The gains are negligible unless one symbol dominates.

- **RLE stores `usize` counts with no maximum**: `RleEntry.count` is a `usize`. If decompression code assumes a smaller count type (e.g., `u16`), it will overflow. The serialized format has no defined count width, making cross-platform interoperability risky.

- **Entropy encoding is trivial bit-packing**: `entropy_encode()` simply packs 2 bits per trit, achieving 2 bits/symbol. This is not actually entropy coding — it doesn't adapt to symbol frequencies. The name is misleading; "bit-packed encoding" would be more accurate.

- **CompressionTracker compares size estimates, not actual compressed output**: `tracker.record()` takes a pre-computed byte count, not the actual compressed data. This means the comparison can be wrong if the size estimates are inaccurate (e.g., not accounting for metadata overhead in Huffman trees or LZW dictionaries).

- **Dictionary compressor requires separate training data**: `DictionaryCompressor` must be trained on representative data before use. If the training data doesn't reflect the actual distribution of the data being compressed, the dictionary entries are useless and may even expand the output.

## Ecosystem

Part of the **SuperInstance** ternary computing crate family:

- `ternary-matrix` — Compact ternary matrix operations
- `ternary-hash` — Hashing and fingerprinting for ternary data
- `ternary-pca` — Principal component analysis on ternary values
- `ternary-ga` — Genetic algorithms with ternary genomes
- `ternary-reservoir` — Echo state networks with ternary nodes
- `ternary-evolution-advanced` — Advanced evolutionary optimization
- `ternary-geometry` — Geometric algorithms in ternary space
- `ternary-causality` — Causal inference for ternary systems
- `ternary-consensus` — Distributed consensus for ternary agents

## License

MIT
