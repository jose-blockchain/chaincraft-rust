#!/bin/bash

# ChainCraft Rust Release Preparation Script
# This script runs all the necessary checks before a release

set -e

echo "🚀 Preparing ChainCraft Rust for release..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "This script must be run from the rust-port directory"
    exit 1
fi

# Get version from Cargo.toml
VERSION=$(grep '^version =' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
echo "Preparing release for version: $VERSION"

# Check if VERSION is provided as argument
if [ "$1" != "" ]; then
    if [ "$1" != "$VERSION" ]; then
        print_warning "Version mismatch: Cargo.toml has $VERSION but you specified $1"
        echo "Do you want to update Cargo.toml to version $1? (y/N)"
        read -r response
        if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
            sed -i.bak "s/version = \"$VERSION\"/version = \"$1\"/" Cargo.toml
            VERSION=$1
            print_status "Updated Cargo.toml to version $VERSION"
        else
            print_error "Aborted due to version mismatch"
            exit 1
        fi
    fi
fi

echo ""
echo "Running pre-release checks..."

# Check formatting
echo "📝 Checking code formatting..."
if ! cargo fmt --check; then
    echo "✗ Code formatting issues found. Run 'cargo fmt' to fix."
    exit 1
fi
echo "✓ Code formatting is correct"

# Run clippy
echo "🔍 Running Clippy..."
if ! RUSTFLAGS="-A unused-variables -A dead-code -A clippy::assertions-on-constants" cargo clippy; then
    echo "✗ Clippy found issues"
    exit 1
fi
echo "✓ Clippy checks passed"

# Run tests
echo "🧪 Running tests..."
if ! cargo test; then
    echo "✗ Tests failed"
    exit 1
fi
echo "✓ All tests passed"

# Run doc tests
echo "📚 Building documentation..."
if ! cargo doc --no-deps; then
    echo "✗ Documentation build failed"
    exit 1
fi
echo "✓ Documentation built successfully"

# Publish check (dry run)
echo "📦 Verifying package for publishing..."
if ! cargo publish --dry-run; then
    echo "✗ Package verification failed"
    exit 1
fi
echo "✓ Package verified for publishing"

echo ""
echo "🎉 All checks passed! Ready for release $VERSION"
echo ""
echo "Next steps:"
echo "1. Commit any remaining changes: git add . && git commit -m \"Prepare release $VERSION\""
echo "2. Create and push tag: git tag v$VERSION && git push origin v$VERSION"
echo "3. Create a GitHub release at: https://github.com/jose-blockchain/chaincraft-rust/releases/new"
echo "4. The GitHub Actions will automatically publish to crates.io"
echo ""
echo "Release checklist:"
echo "□ Tag created and pushed"
echo "□ GitHub release created with changelog"
echo "□ Crates.io publish successful"
echo "□ Documentation updated on docs.rs"
echo "□ Announcement prepared"

exit 0 