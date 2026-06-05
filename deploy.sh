#!/usr/bin/env bash
set -euo pipefail

# Deploy the built site to the `gh-pages` branch.
#
# Builds the site into `dist/` with the Rust SSG, then publishes the contents
# of `dist/` to the `gh-pages` branch as a single fresh commit (force-pushed).
# Uses a temporary git worktree so your current branch and working tree are
# left untouched.

DEPLOY_BRANCH="gh-pages"
REMOTE="origin"
CUSTOM_DOMAIN="orellazri.com"

# Run from the repo root (this script's directory).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Refuse to deploy with a dirty working tree.
if [[ -n "$(git status --porcelain)" ]]; then
  echo "error: working tree is dirty. Commit or stash your changes first." >&2
  exit 1
fi

SOURCE_SHA="$(git rev-parse --short HEAD)"

echo "==> Building site..."
cargo run --release -- build

if [[ ! -d dist ]]; then
  echo "error: build did not produce a dist/ directory." >&2
  exit 1
fi

# Written after build because build() wipes dist/ on every run.
echo "$CUSTOM_DOMAIN" > dist/CNAME
touch dist/.nojekyll

echo "==> Publishing to '$DEPLOY_BRANCH'..."

# Best-effort fetch so we update an existing remote branch in place.
git fetch "$REMOTE" "$DEPLOY_BRANCH" || true

WORKTREE_DIR="$(mktemp -d)"
cleanup() {
  git worktree remove --force "$WORKTREE_DIR" >/dev/null 2>&1 || true
  rm -rf "$WORKTREE_DIR"
}
trap cleanup EXIT

# Check out gh-pages into the worktree. Prefer the local branch, then the
# remote-tracking branch, otherwise create an orphan branch.
if git show-ref --verify --quiet "refs/heads/$DEPLOY_BRANCH"; then
  git worktree add "$WORKTREE_DIR" "$DEPLOY_BRANCH"
elif git show-ref --verify --quiet "refs/remotes/$REMOTE/$DEPLOY_BRANCH"; then
  git worktree add -B "$DEPLOY_BRANCH" "$WORKTREE_DIR" "$REMOTE/$DEPLOY_BRANCH"
else
  git worktree add --detach "$WORKTREE_DIR"
  git -C "$WORKTREE_DIR" checkout --orphan "$DEPLOY_BRANCH"
fi

# Replace the worktree contents with the freshly built site.
git -C "$WORKTREE_DIR" rm -rfq --ignore-unmatch . >/dev/null 2>&1 || true
# Remove any leftover untracked files too.
find "$WORKTREE_DIR" -mindepth 1 -maxdepth 1 ! -name '.git' -exec rm -rf {} +
cp -R dist/. "$WORKTREE_DIR"/

git -C "$WORKTREE_DIR" add -A

if git -C "$WORKTREE_DIR" diff --cached --quiet; then
  echo "==> No changes to deploy."
  exit 0
fi

git -C "$WORKTREE_DIR" commit -q -m "deploy: ${SOURCE_SHA} $(date -u +%Y-%m-%dT%H:%M:%SZ)"
git -C "$WORKTREE_DIR" push --force "$REMOTE" "$DEPLOY_BRANCH"

echo "==> Deployed to '$DEPLOY_BRANCH' (from ${SOURCE_SHA})."
