#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE_TAG="${IMAGE_TAG:-chat2responses:local}"
SKIP_DOCKER=0
PUSH=0
PUSH_LATEST=1
PLATFORMS="${PLATFORMS:-linux/amd64,linux/arm64}"
BUILDER="${BUILDER:-newapi-builder}"
DOCKERHUB_USERNAME="${DOCKERHUB_USERNAME:-}"
DOCKERHUB_REPO="${DOCKERHUB_REPO:-lucentttt/chat2responses}"
DOCKERHUB_TOKEN="${DOCKERHUB_TOKEN:-}"
DOCKERHUB_TOKEN_STDIN=0

usage() {
  cat <<'USAGE'
Usage: scripts/package-docker.sh [options]

Build and package chat2responses into a Docker image.
The Dockerfile builds the Rust binary and admin UI inside target-platform
builder containers, so multi-architecture images contain matching binaries.

Options:
  --image-tag TAG   Docker image tag. Default: chat2responses:local
  --skip-docker    Prepare artifacts only; do not run docker build.
  --push           Push the image to Docker Hub after building.
  --no-latest      With --push, do not also tag and push :latest.
  --platforms LIST  Build platforms. Default: linux/amd64,linux/arm64
                   Can also use PLATFORMS.
  --builder NAME    Docker buildx builder. Default: newapi-builder
                   Can also use BUILDER.
  --dockerhub-username USER
                   Docker Hub username. Can also use DOCKERHUB_USERNAME.
  --dockerhub-repo REPO
                   Docker Hub repository, for example myuser/chat2responses.
                   Default: lucentttt/chat2responses. Can also use DOCKERHUB_REPO.
  --dockerhub-token-stdin
                   Read Docker Hub token from stdin for docker login.
                   Can also use DOCKERHUB_TOKEN.
  -h, --help       Show this help.

Examples:
  scripts/package-docker.sh
  scripts/package-docker.sh --image-tag chat2responses:v0.2.0
  DOCKERHUB_USERNAME=lucentttt DOCKERHUB_TOKEN=... scripts/package-docker.sh --push --image-tag v0.2.0
  DOCKERHUB_USERNAME=lucentttt DOCKERHUB_TOKEN=... scripts/package-docker.sh --push --image-tag v0.2.0 --no-latest
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --image-tag)
      IMAGE_TAG="${2:?missing value for --image-tag}"
      shift 2
      ;;
    --platforms)
      PLATFORMS="${2:?missing value for --platforms}"
      shift 2
      ;;
    --builder)
      BUILDER="${2:?missing value for --builder}"
      shift
      ;;
    --skip-docker)
      SKIP_DOCKER=1
      shift
      ;;
    --push)
      PUSH=1
      shift
      ;;
    --no-latest)
      PUSH_LATEST=0
      shift
      ;;
    --dockerhub-username)
      DOCKERHUB_USERNAME="${2:?missing value for --dockerhub-username}"
      shift 2
      ;;
    --dockerhub-repo)
      DOCKERHUB_REPO="${2:?missing value for --dockerhub-repo}"
      shift 2
      ;;
    --dockerhub-token-stdin)
      DOCKERHUB_TOKEN_STDIN=1
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

if [[ "$PUSH" -eq 1 && "$SKIP_DOCKER" -eq 1 ]]; then
  echo "--push cannot be used with --skip-docker." >&2
  exit 2
fi

if [[ "$PUSH" -eq 1 ]]; then
  if [[ -n "$DOCKERHUB_REPO" ]]; then
    if [[ "$IMAGE_TAG" == *":"* ]]; then
      TAG_PART="${IMAGE_TAG##*:}"
    else
      TAG_PART="$IMAGE_TAG"
    fi
    IMAGE_TAG="$DOCKERHUB_REPO:$TAG_PART"
  elif [[ "$IMAGE_TAG" != *"/"* ]]; then
    if [[ -z "$DOCKERHUB_USERNAME" ]]; then
      echo "Pushing chat2responses:local would not target your Docker Hub namespace." >&2
      echo "Use --dockerhub-repo USER/chat2responses or --image-tag USER/chat2responses:TAG." >&2
      exit 2
    fi
    if [[ "$IMAGE_TAG" == *":"* ]]; then
      TAG_PART="${IMAGE_TAG##*:}"
    else
      TAG_PART="$IMAGE_TAG"
    fi
    IMAGE_TAG="$DOCKERHUB_USERNAME/chat2responses:$TAG_PART"
  fi
fi

LATEST_IMAGE_TAG=""
if [[ "$PUSH" -eq 1 && "$PUSH_LATEST" -eq 1 ]]; then
  LAST_COMPONENT="${IMAGE_TAG##*/}"
  if [[ "$LAST_COMPONENT" == *":"* ]]; then
    IMAGE_REPOSITORY="${IMAGE_TAG%:*}"
  else
    IMAGE_REPOSITORY="$IMAGE_TAG"
  fi
  LATEST_IMAGE_TAG="$IMAGE_REPOSITORY:latest"
  if [[ "$LATEST_IMAGE_TAG" == "$IMAGE_TAG" ]]; then
    LATEST_IMAGE_TAG=""
  fi
fi

if [[ "$SKIP_DOCKER" -eq 1 ]]; then
  echo "==> Docker build skipped"
  echo "Dockerfile builds release artifacts inside target-platform builder containers."
  exit 0
fi

if [[ "$PUSH" -eq 1 ]]; then
  if [[ "$DOCKERHUB_TOKEN_STDIN" -eq 1 ]]; then
    if [[ -z "$DOCKERHUB_USERNAME" ]]; then
      echo "--dockerhub-token-stdin requires --dockerhub-username or DOCKERHUB_USERNAME." >&2
      exit 2
    fi
    echo "==> Logging in to Docker Hub as $DOCKERHUB_USERNAME"
    docker login --username "$DOCKERHUB_USERNAME" --password-stdin
  elif [[ -n "$DOCKERHUB_TOKEN" ]]; then
    if [[ -z "$DOCKERHUB_USERNAME" ]]; then
      echo "DOCKERHUB_TOKEN requires --dockerhub-username or DOCKERHUB_USERNAME." >&2
      exit 2
    fi
    echo "==> Logging in to Docker Hub as $DOCKERHUB_USERNAME"
    printf '%s' "$DOCKERHUB_TOKEN" | docker login --username "$DOCKERHUB_USERNAME" --password-stdin
  else
    echo "==> Using existing Docker login session"
  fi

  TAG_ARGS=(-t "$IMAGE_TAG")
  if [[ -n "$LATEST_IMAGE_TAG" ]]; then
    TAG_ARGS+=(-t "$LATEST_IMAGE_TAG")
    echo "==> Building and pushing Docker images $IMAGE_TAG and $LATEST_IMAGE_TAG for $PLATFORMS"
  else
    echo "==> Building and pushing Docker image $IMAGE_TAG for $PLATFORMS"
  fi
  docker buildx build \
    --builder "$BUILDER" \
    --platform "$PLATFORMS" \
    --provenance=false \
    "${TAG_ARGS[@]}" \
    --push \
    .
else
  if [[ "$PLATFORMS" == *","* ]]; then
    LOCAL_PLATFORM="$(docker version -f '{{.Server.Os}}/{{.Server.Arch}}')"
    echo "==> --push not set; building local single-platform image for $LOCAL_PLATFORM"
    echo "==> Use --push to publish multi-platform image for $PLATFORMS"
    docker build --platform "$LOCAL_PLATFORM" -t "$IMAGE_TAG" .
  else
    echo "==> Building Docker image $IMAGE_TAG for $PLATFORMS"
    docker build --platform "$PLATFORMS" -t "$IMAGE_TAG" .
  fi
fi

echo "==> Done"
echo "Run with:"
echo "docker run -d --name chat2responses -p 4444:4444 -e CHAT2RESPONSES_SECRET='change-this-long-random-secret' -v chat2responses-data:/app/data $IMAGE_TAG"
