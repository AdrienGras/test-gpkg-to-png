
## Design Documents

- Full design specification: `docs/plans/2026-02-01-gpkg-to-png-design.md`
- Implementation lessons learned: `LESSONS.md`

## CI/CD

### Release Workflow

**Trigger:** Git tags matching `v*` (e.g., `v1.2.3`)

**Platforms Built:**
- Linux AMD64 (x86_64-unknown-linux-gnu)
- Linux ARM64 (aarch64-unknown-linux-gnu)
- macOS Intel (x86_64-apple-darwin)
- macOS Apple Silicon (aarch64-apple-darwin)

**Workflow:**
1. Four parallel build jobs (one per platform)
2. Each job builds release binary and uploads artifact
3. Release job waits for all builds, creates GitHub release
4. Release includes all binaries + checksums.txt

> **Note**: Windows builds are intentionally excluded due to persistent linking issues with `proj` and `sqlite3` on that platform.

**Manual Testing:**
```bash
# Create test tag
git tag v0.0.0-test
git push origin v0.0.0-test

# Wait for CI, then delete test tag and release
git tag -d v0.0.0-test
git push origin :refs/tags/v0.0.0-test
```
