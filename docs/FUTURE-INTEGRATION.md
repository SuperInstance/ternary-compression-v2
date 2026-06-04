# Future Integration: ternary-compression-v2

## Current State
Provides advanced ternary compression: run-length encoding, ternary Huffman coding, LZW for ternary alphabet, dictionary compression, entropy coding, and compression ratio tracking for streams of {-1, 0, +1}.

## Integration Opportunities

### With ternary-database (State Storage)
Database stores room histories as ternary state sequences. Compression reduces storage by 5-50x depending on entropy. `rle_compress` for sequences with runs (common in stable rooms). `ternary_huffman` for irregular patterns. `lzw_ternary` for repetitive patterns across rooms.

### With ternary-protocol (Message Compression)
Protocol messages carry ternary payloads. Bandwidth is precious on ESP32/Jetson uplinks. Compress messages with `ternary-compression-v2` before transmission, decompress on receipt. Dictionary compression allows shared dictionaries between rooms — compress more once both sides know the vocabulary.

### With ternary-spreadsheet
Spreadsheet cells are ternary values. Entire spreadsheets compress well because cell grids have spatial correlation (neighboring cells tend to have similar values). LZW compression on row-major cell data achieves high compression ratios.

## Potential in Mature Systems
In room-as-codespace, room state snapshots are ternary data that needs efficient storage and transmission. Compression minimizes Codespace memory usage, reduces network bandwidth for room synchronization via PLATO, and speeds up room state restoration. Entropy coding tracks how much information a room actually contains — low entropy rooms are routine, high entropy rooms are novel.

## Cross-Pollination Ideas
- Entropy as a room novelty metric — high compression ratio = predictable room, low ratio = surprising room
- Dictionary compression with shared dictionaries across rooms in the same campus
- Compression ratio as a health indicator — suddenly incompressible room state means something changed

## Dependencies for Next Steps
- ternary-database needs compressed storage backend
- ternary-protocol needs message compression/decompression layer
- Integration with ternary-spreadsheet for grid compression
