# Ternary Compression v2 — Advanced Compression for {-1, 0, +1} Alphabets

**Ternary Compression v2** provides five compression algorithms — run-length encoding (RLE), ternary Huffman coding, LZW adapted for ternary alphabets, dictionary compression, and entropy coding — all designed specifically for streams of balanced-ternary values in {-1, 0, +1}. It tracks compression ratios and provides round-trip lossless decompression for every algorithm.

## Why It Matters

Ternary neural networks store weights as trits, and a single model may contain billions of them. Compressing ternary streams is essential for model distribution, federated weight transfer over low-bandwidth links, and on-device inference where memory is constrained. Standard binary compressors operate on bytes and underperform on ternary data because they ignore the 3-state structure. By working natively in base-3, this crate achieves better compression ratios for ternary weight matrices, kernel traces, and agent state logs.

## How It Works

### Run-Length Encoding (RLE)

Compresses consecutive identical trits into `(trit, count)` pairs. For a stream of length n with k runs, compression produces O(k) entries. Best case O(1) space (all identical), worst case O(n) (alternating). Decompression is O(n).

### Ternary Huffman Coding

Builds an optimal prefix-free code over the ternary alphabet using a 3-ary Huffman tree (each internal node has exactly 3 children). This is the ternary analog of binary Huffman: instead of combining the 2 least-frequent symbols, we combine the 3 least-frequent. The expected codeword length L satisfies:

```
H₃(X) ≤ L < H₃(X) + 1
```

where H₃(X) = -Σ pᵢ log₃(pᵢ) is the ternary entropy. Tree construction is O(n log n) using a priority queue.

### LZW for Ternary Alphabets

Adapts the Lempel-Ziv-Welch algorithm: the initial dictionary contains the 3 single-trit symbols {-1, 0, +1}. As the encoder reads the stream, it builds multi-trit entries. When a new trit would extend the current match beyond the dictionary, it emits the current code and inserts the extended entry. Code table grows dynamically. Encoding and decoding are both O(n).

### Dictionary Compression

Identifies repeated multi-trit patterns and replaces them with dictionary indices. The dictionary is built from frequency analysis of n-gram patterns. Effective when the stream contains repeated structural motifs (e.g., weight blocks in ternary networks).

### Entropy Coding

Computes the Shannon entropy of the ternary stream: H = -Σ pᵢ log₂(pᵢ) for i ∈ {-1, 0, +1}. This establishes the theoretical compression limit and guides selection of which algorithm to apply.

## Quick Start

```rust
use ternary_compression_v2::{Trit, rle_compress, rle_decompress};

let data = vec![
    Trit::Pos, Trit::Pos, Trit::Pos, Trit::Zero, Trit::Pos,
    Trit::Neg, Trit::Neg, Trit::Pos,
];

// Run-length encode
let compressed = rle_compress(&data);
assert_eq!(compressed.len(), 5); // 5 runs

// Decompress
let restored = rle_decompress(&compressed);
assert_eq!(restored, data);
```

```bash
cargo add ternary-compression-v2
```

## API

| Type / Function | Description |
|---|---|
| `Trit` | Enum: `Neg(-1)`, `Zero(0)`, `Pos(+1)` |
| `rle_compress(&[Trit]) → Vec<RleEntry>` | Run-length encode (O(n)) |
| `rle_decompress(&[RleEntry]) → Vec<Trit>` | RLE decode (O(n)) |
| `RleEntry` | `{ trit: Trit, count: usize }` |

## Architecture Notes

In the **SuperInstance** ecosystem, this crate compresses ternary weight matrices and agent state vectors for transfer between fleet nodes. Weight compression directly impacts the η (entropy) term of γ + η = C: better compression means lower entropy overhead during fleet synchronization. See [Architecture](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md).

## References

- Huffman, David A. "A Method for the Construction of Minimum-Redundancy Codes," *Proceedings of the IRE*, 1952.
- Welch, Terry. "A Technique for High-Performance Data Compression," *IEEE Computer*, 17(6), 1984 — the LZW algorithm.
- Cover, Thomas & Thomas, Joy. *Elements of Information Theory*, 2nd ed., Wiley, 2006 — entropy bounds.

## License

MIT
