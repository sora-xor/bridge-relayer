# Bridge Relayer — Release Checklist

This checklist converts a local-dev build (using `../sora2-common` path overrides) into a reproducible release/CI build pinned to `sora2-common` tags or specific commits, and back again if needed.

## Pre‑flight

- Ensure the code builds locally in dev mode:
  - `cargo build -p bridge-relayer -r`
- Choose a sora2-common version to pin:
  - Tag example: `export SORA2_COMMON_TAG="vX.Y.Z"`
  - Or specific commit: `export SORA2_COMMON_REV="<40-char-sha>"`

## 1) Remove local path overrides (root Cargo.toml)

Delete the `[patch."https://github.com/sora-xor/sora2-common.git"]` block that points to `../sora2-common`.

macOS (BSD sed):

```
sed -i '' '/\[patch."https:\/\/github.com\/sora-xor\/sora2-common.git"\]/,/^$/d' Cargo.toml
```

Linux (GNU sed):

```
sed -i '/\[patch."https:\/\/github.com\/sora-xor\/sora2-common.git"\]/,/^$/d' Cargo.toml
```

## 2) Switch relayer crate to pinned git deps

Replace path deps in `relayer/Cargo.toml` with pinned git deps. Use either TAG or REV (choose one form consistently).

macOS (TAG):

```
for CR in beefy-light-client bridge-common bridge-types leaf-provider-rpc; do \
  sed -i '' "s|$CR = { path = \"../../sora2-common/pallets/[^\"]*\" }|$CR = { git = \"https://github.com/sora-xor/sora2-common.git\", tag = \"$SORA2_COMMON_TAG\" }|g" relayer/Cargo.toml; \
done
```

macOS (REV):

```
for CR in beefy-light-client bridge-common bridge-types leaf-provider-rpc; do \
  sed -i '' "s|$CR = { path = \"../../sora2-common/pallets/[^\"]*\" }|$CR = { git = \"https://github.com/sora-xor/sora2-common.git\", rev = \"$SORA2_COMMON_REV\" }|g" relayer/Cargo.toml; \
done
```

Linux: replace `sed -i ''` with `sed -i`.

## 3) Pin gen crates to the same sora2-common version

Add `tag` or `rev` to these files (they already depend on sora2-common via git):

- `substrate-gen/Cargo.toml`
- `parachain-gen/Cargo.toml`
- `liberland-gen/Cargo.toml`

macOS (TAG):

```
for F in substrate-gen/Cargo.toml parachain-gen/Cargo.toml liberland-gen/Cargo.toml; do \
  sed -i '' "s|= { git = \"https://github.com/sora-xor/sora2-common.git\" }|= { git = \"https://github.com/sora-xor/sora2-common.git\", tag = \"$SORA2_COMMON_TAG\" }|g" "$F"; \
done
```

macOS (REV):

```
for F in substrate-gen/Cargo.toml parachain-gen/Cargo.toml liberland-gen/Cargo.toml; do \
  sed -i '' "s|= { git = \"https://github.com/sora-xor/sora2-common.git\" }|= { git = \"https://github.com/sora-xor/sora2-common.git\", rev = \"$SORA2_COMMON_REV\" }|g" "$F"; \
done
```

Linux: replace `sed -i ''` with `sed -i`.

## 4) Verify no path overrides remain

```
rg -n '../sora2-common|sora2-common.*path' -S
```

The command should produce no matches in `Cargo.toml` files.

## 5) Refresh lockfile and build

```
cargo update -p beefy-light-client -p bridge-common -p bridge-types -p leaf-provider-rpc
cargo build -p bridge-relayer -r
cargo test --no-run
```

If there are API mismatches, verify the chosen tag/rev matches the relayer’s expected TON types and palette APIs, and pin to a compatible version.

## 6) Update docs

- `README.md` → Replace placeholder tag/rev with the actual pinned value in the Release/CI build section.
- `status.md` → Update the Dependencies section to reflect pinned tags (no local overrides).
- `roadmap.md` → Mark the P0 dependency task as done when CI builds from tags.

## 7) Commit, tag, push

```
git add -A
git commit -m "release: pin sora2-common ${SORA2_COMMON_TAG:-$SORA2_COMMON_REV} + docs"
git tag -a bridge-relayer-vA.B.C -m "Bridge Relayer vA.B.C"
git push --follow-tags
```

Update `A.B.C` appropriately following your versioning scheme.

## 8) CI sanity

- Ensure CI runners do not have a sibling `../sora2-common` checkout.
- Jenkinsfile/CI should run a clean build using only the pinned git deps.

## 9) Toggle back to dev (optional)

To restore local dev overrides:

1) Re-add the patch block in root `Cargo.toml` with relative paths to `../sora2-common/pallets/*`.
2) In `relayer/Cargo.toml`, switch the four `sora2-common` crates back to `path = "../../sora2-common/pallets/..."`.
3) `cargo build -p bridge-relayer -r` to verify.

