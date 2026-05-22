#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION_FILE="${VERSION_FILE:-$ROOT_DIR/VERSION}"
GITHUB_REPO="${GITHUB_REPO:-lucent404/Chat2Responses}"
DOCKERHUB_REPO="${DOCKERHUB_REPO:-lucentttt/chat2responses}"
SKIP_DOCKER=0
SKIP_GITHUB=0
NO_LATEST=0
DRY_RUN=0

usage() {
  cat <<'USAGE'
Usage: scripts/release.sh [options]

Create a project release from the VERSION file:
  1. Push Docker image tags DOCKERHUB_REPO:v<version> and :latest.
  2. Create GitHub release v<version> with generated notes.

Options:
  --skip-docker    Do not build or push Docker images.
  --skip-github    Do not create the GitHub release.
  --no-latest      Do not push the Docker :latest tag.
  --dry-run        Print commands without executing them.
  -h, --help       Show this help.

Environment:
  VERSION_FILE     Version file path. Default: ./VERSION
  GITHUB_REPO      GitHub owner/repo. Default: lucent404/Chat2Responses
  DOCKERHUB_REPO   Docker Hub repository. Default: lucentttt/chat2responses
  DOCKERHUB_USERNAME  Optional Docker Hub username passed through to package-docker.sh
  DOCKERHUB_TOKEN  Optional Docker Hub token passed through to package-docker.sh
  PLATFORMS        Optional Docker platforms passed through to package-docker.sh
  BUILDER          Optional buildx builder passed through to package-docker.sh

Examples:
  scripts/release.sh --dry-run
  scripts/release.sh
  scripts/release.sh --skip-docker
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-docker)
      SKIP_DOCKER=1
      shift
      ;;
    --skip-github)
      SKIP_GITHUB=1
      shift
      ;;
    --no-latest)
      NO_LATEST=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

cd "$ROOT_DIR"

if [[ ! -f "$VERSION_FILE" ]]; then
  echo "Version file not found: $VERSION_FILE" >&2
  exit 1
fi

VERSION="$(tr -d '[:space:]' < "$VERSION_FILE")"
if [[ ! "$VERSION" =~ ^[0-9]+[.][0-9]+[.][0-9]+([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "Invalid VERSION value: $VERSION" >&2
  echo "Expected semantic version like 0.2.0 or 0.2.0-rc.1" >&2
  exit 1
fi

TAG="v$VERSION"

run() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    printf '+'
    printf ' %q' "$@"
    printf '\n'
  else
    "$@"
  fi
}

if [[ "$SKIP_DOCKER" -eq 0 ]]; then
  DOCKER_ARGS=(scripts/package-docker.sh --push --dockerhub-repo "$DOCKERHUB_REPO" --image-tag "$TAG")
  if [[ "$NO_LATEST" -eq 1 ]]; then
    DOCKER_ARGS+=(--no-latest)
  fi
  run "${DOCKER_ARGS[@]}"
fi

if [[ "$SKIP_GITHUB" -eq 0 ]]; then
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "+ gh release view $TAG --repo $GITHUB_REPO"
  elif gh release view "$TAG" --repo "$GITHUB_REPO" >/dev/null 2>&1; then
    echo "GitHub release already exists: $TAG" >&2
    echo "Delete it first or update VERSION." >&2
    exit 1
  fi
  run gh release create "$TAG" --repo "$GITHUB_REPO" --title "$TAG" --generate-notes
fi

echo "==> Release complete: $TAG"
