# Empty Type MultiPolygon Support Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Handle malformed GeoJSON files where `"type":""` should be interpreted as `"type":"MultiPolygon"`.

**Architecture:** Pre-processing string replacement in `GeojsonReader::open()` before parsing, following the existing pattern for CSV double-quote escaping (line 32).

**Tech Stack:** Rust, geojson crate, TDD with cargo test

---

## Task 1: Test for empty type in root geometry

**Files:**
- Modify: `src/geojson.rs:185-296` (add test in existing test module)

**Step 1: Write the failing test**

Add this test after `test_parse_raw_multipolygon` (around line 231):

```rust
#[test]
fn test_parse_empty_type_root_geometry() {
    let json = r#"{
        "type": "",
        "coordinates": [[[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]]
    }"#;

    let geojson: GeoJson = json.parse().unwrap();
    let geometries = extract_geometries(&geojson);
    assert_eq!(geometries.len(), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_parse_empty_type_root_geometry`

Expected: FAIL with parse error because `"type":""` is not valid GeoJSON

**Step 3: Implement the fix**

In `GeojsonReader::open()` method, add after line 32 (after the CSV fix):

```rust
// Fix CSV-style double-quote escaping (""type"" -> "type")
// This handles malformed GeoJSON exported from some tools
let content = content.replace("\"\"", "\"");

// Fix empty type field ("type":"" -> "type":"MultiPolygon")
// Handles malformed GeoJSON where type field is empty
let content = content.replace(r#""type":"""#, r#""type":"MultiPolygon""#);

let geojson: GeoJson = content.parse().map_err(|e| {
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_parse_empty_type_root_geometry`

Expected: PASS

**Step 5: Run all tests to ensure no regression**

Run: `cargo test`

Expected: All 49 tests pass (48 existing + 1 new)

**Step 6: Commit**

```bash
git add src/geojson.rs
git commit -m "test: add test for empty type in root geometry"
```

---

## Task 2: Test for empty type in Feature geometry

**Files:**
- Modify: `src/geojson.rs:185-296` (add test in existing test module)

**Step 1: Write the failing test**

Add this test after `test_parse_empty_type_root_geometry`:

```rust
#[test]
fn test_parse_empty_type_in_feature() {
    let json = r#"{
        "type": "Feature",
        "geometry": {
            "type": "",
            "coordinates": [[[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]]
        }
    }"#;

    let geojson: GeoJson = json.parse().unwrap();
    let geometries = extract_geometries(&geojson);
    assert_eq!(geometries.len(), 1);
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test test_parse_empty_type_in_feature`

Expected: PASS (the fix from Task 1 already handles this case)

**Step 3: Run all tests to ensure no regression**

Run: `cargo test`

Expected: All 50 tests pass (48 original + 2 new)

**Step 4: Commit**

```bash
git add src/geojson.rs
git commit -m "test: add test for empty type in Feature geometry"
```

---

## Task 3: Test for empty type in FeatureCollection with mixed types

**Files:**
- Modify: `src/geojson.rs:185-296` (add test in existing test module)

**Step 1: Write the comprehensive test**

Add this test after `test_parse_empty_type_in_feature`:

```rust
#[test]
fn test_parse_empty_type_in_featurecollection() {
    let json = r#"{
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "geometry": {
                    "type": "",
                    "coordinates": [[[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]]
                }
            },
            {
                "type": "Feature",
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[2.0, 2.0], [3.0, 2.0], [3.0, 3.0], [2.0, 3.0], [2.0, 2.0]]]
                }
            },
            {
                "type": "Feature",
                "geometry": {
                    "type": "",
                    "coordinates": [[[[4.0, 4.0], [5.0, 4.0], [5.0, 5.0], [4.0, 5.0], [4.0, 4.0]]]]
                }
            }
        ]
    }"#;

    let geojson: GeoJson = json.parse().unwrap();
    let geometries = extract_geometries(&geojson);
    assert_eq!(geometries.len(), 3); // All three features should be parsed
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test test_parse_empty_type_in_featurecollection`

Expected: PASS (the fix handles multiple occurrences)

**Step 3: Run all tests to ensure no regression**

Run: `cargo test`

Expected: All 51 tests pass (48 original + 3 new)

**Step 4: Commit**

```bash
git add src/geojson.rs
git commit -m "test: add test for empty type in FeatureCollection with mixed types"
```

---

## Task 4: Final verification and implementation commit

**Files:**
- Verify: `src/geojson.rs:21-45` (implementation is already done in Task 1)

**Step 1: Review the implementation**

Verify that lines 32-36 in `src/geojson.rs` contain:

```rust
// Fix CSV-style double-quote escaping (""type"" -> "type")
// This handles malformed GeoJSON exported from some tools
let content = content.replace("\"\"", "\"");

// Fix empty type field ("type":"" -> "type":"MultiPolygon")
// Handles malformed GeoJSON where type field is empty
let content = content.replace(r#""type":"""#, r#""type":"MultiPolygon""#);
```

**Step 2: Run full test suite**

Run: `cargo test`

Expected: All 51 tests pass (48 original + 3 new)

**Step 3: Run integration tests**

Run: `cargo test --test integration -- --ignored`

Expected: All integration tests pass (no regression)

**Step 4: Commit the implementation**

```bash
git add src/geojson.rs
git commit -m "feat(geojson): support empty type field as MultiPolygon

Handles malformed GeoJSON files where geometry type field is empty.
Pre-processing replaces \"type\":\"\" with \"type\":\"MultiPolygon\"
before parsing, following the pattern established for CSV escaping.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Update documentation

**Files:**
- Modify: `CLAUDE.md:293-382` (Lessons Learned section)

**Step 1: Add entry to Lessons Learned**

Add a new subsection in the "Lessons Learned" section after the GeoJSON feature entry:

```markdown
### Feature: Empty Type MultiPolygon Support (2026-02-02)

#### ✅ What Worked Well

**1. Following Established Patterns**
- String replacement approach consistent with CSV fix (line 32)
- Minimal code change (3 lines + tests)
- Zero risk of regression

**2. Test-Driven Development**
- Three tests covering all use cases (root, Feature, FeatureCollection)
- Tests written before implementation confirmed
- All tests pass on first run after implementation

**3. Simple Solution Over Complex**
- Rejected parsing with serde_json (too complex)
- Rejected regex approach (unnecessary)
- String replacement sufficient and maintainable

#### 🎯 Best Practices Identified

**1. Pattern Consistency**
- New malformed data fixes should follow established patterns
- Comment style matches existing code
- Placement logical (with other pre-processing)

**2. Comprehensive Test Coverage**
- Test root geometry, Feature, and FeatureCollection
- Test mixed valid/invalid types
- Verify all existing tests still pass

#### 💡 Key Insight

**"Follow the grain of the codebase"** - When a pattern exists for similar problems (CSV escaping), extend it rather than invent new approaches. Consistency beats novelty.

---
```

**Step 2: Verify documentation formatting**

Run: `cat CLAUDE.md | grep -A 10 "Empty Type MultiPolygon"`

Expected: New section appears correctly formatted

**Step 3: Commit documentation**

```bash
git add CLAUDE.md
git commit -m "docs: add lessons learned for empty type support

Documents the simple string replacement approach and pattern
consistency as key to maintainable malformed data handling.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 6: Verify and finalize

**Files:**
- Verify: All files modified

**Step 1: Run full test suite one more time**

Run: `cargo test`

Expected: All 51 tests pass

**Step 2: Run clippy for code quality**

Run: `cargo clippy`

Expected: No new warnings (same as baseline)

**Step 3: Build release binary**

Run: `cargo build --release`

Expected: Successful build with no errors

**Step 4: Review git log**

Run: `git log --oneline -6`

Expected: See all 5 commits in order:
1. test: add test for empty type in root geometry
2. test: add test for empty type in Feature geometry
3. test: add test for empty type in FeatureCollection with mixed types
4. feat(geojson): support empty type field as MultiPolygon
5. docs: add lessons learned for empty type support

**Step 5: Verify worktree is clean**

Run: `git status`

Expected: "nothing to commit, working tree clean"

---

## Summary

**Total Tasks**: 6
**Estimated Time**: 20-30 minutes
**Files Modified**:
- `src/geojson.rs` (implementation + 3 tests)
- `CLAUDE.md` (documentation)

**Test Coverage**:
- 3 new unit tests
- All existing tests remain passing
- Integration tests verified

**Principles Applied**:
- TDD (tests before confirming implementation)
- DRY (follows existing pattern)
- YAGNI (simplest solution that works)
- Frequent commits (6 total)
