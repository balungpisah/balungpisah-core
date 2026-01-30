#!/bin/bash

set -e  # Exit on any error

# Function to auto-detect environment from git branch
auto_detect_environment() {
    local current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

    case "$current_branch" in
        "staging")
            echo "staging"
            ;;
        "main")
            echo "production"
            ;;
        *)
            echo ""
            ;;
    esac
}

# Check if environment parameter is provided, otherwise auto-detect
DEPLOY_ENV="${1}"

# If first argument looks like an option (starts with -), then auto-detect environment
if [[ -z "$DEPLOY_ENV" ]] || [[ "$DEPLOY_ENV" == -* ]]; then
    AUTO_ENV=$(auto_detect_environment)

    if [[ -n "$AUTO_ENV" ]]; then
        DEPLOY_ENV="$AUTO_ENV"
        echo -e "\033[0;32m[AUTO-DETECT]\033[0m Environment auto-detected from branch: $DEPLOY_ENV"
        echo ""
    else
        CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
        echo -e "\033[0;31m[ERROR]\033[0m Cannot auto-detect environment from branch: $CURRENT_BRANCH"
        echo ""
        echo "Auto-detection only works for branches: staging, main"
        echo ""
        echo "Usage: $0 [environment] [OPTIONS]"
        echo ""
        echo "Environments:"
        echo "  staging      Deploy to staging environment"
        echo "  production   Deploy to production environment"
        echo "  (optional)   Auto-detected if on 'staging' or 'main' branch"
        echo ""
        echo "Options:"
        echo "  -h, --help          Show help message"
        echo "  -n, --no-push       Build image but don't push to registry"
        echo "  -l, --local-only    Build image only (no push, no deploy)"
        echo "  -t, --tag TAG       Use custom tag instead of branch-based tag"
        echo "  -v, --verify-only   Only verify current deployment status"
        echo ""
        echo "Examples:"
        echo "  $0                          # Auto-detect from current branch"
        echo "  $0 staging                  # Explicitly deploy to staging"
        echo "  $0 production               # Explicitly deploy to production"
        echo "  $0 --local-only             # Auto-detect and build locally only"
        echo "  $0 staging --local-only     # Build for staging locally"
        exit 1
    fi
else
    # Environment was explicitly provided, validate it
    if [[ "$DEPLOY_ENV" != "staging" && "$DEPLOY_ENV" != "production" ]]; then
        echo -e "\033[0;31m[ERROR]\033[0m Invalid environment: $DEPLOY_ENV"
        echo "Valid environments: staging, production"
        exit 1
    fi
    # Remove environment argument from positional parameters
    shift
fi

# Load deployment configuration based on environment
DEPLOY_CONFIG=".env.deploy.${DEPLOY_ENV}"
if [[ ! -f "$DEPLOY_CONFIG" ]]; then
    echo -e "\033[0;31m[ERROR]\033[0m Deployment config not found: $DEPLOY_CONFIG"
    echo "Available configs:"
    ls -1 .env.deploy.* 2>/dev/null | sed 's/^/  - /' || echo "  No .env.deploy.* files found"
    echo ""
    echo "Please create: $DEPLOY_CONFIG"
    echo "You can copy from .env.deploy.example as a template"
    exit 1
fi

export $(cat "$DEPLOY_CONFIG" | grep -v '^#' | sed 's/#.*//' | grep '=' | xargs)
echo -e "\033[0;34m[INFO]\033[0m Loaded deployment config: $DEPLOY_CONFIG"
echo -e "\033[0;34m[INFO]\033[0m Target VPS: $VPS_HOST"
echo -e "\033[0;34m[INFO]\033[0m Deploy path: $COMPOSE_PATH"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REGISTRY="ghcr.io"
REPOSITORY="balungpisah/balungpisah-core"
IMAGE_NAME="${REGISTRY}/${REPOSITORY}"
DOCKERFILE="./Dockerfile.deploy"

# VPS Configuration - These values are loaded from .env.deploy.* file
VPS_HOST="${VPS_HOST:-}"
VPS_USER="${VPS_USER:-}"
VPS_KEY="${VPS_KEY:-}"
COMPOSE_PATH="${COMPOSE_PATH:-}"

# Cross-compilation target
CROSS_TARGET="x86_64-unknown-linux-gnu"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if required tools are installed
check_dependencies() {
    print_status "Checking dependencies..."

    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed or not in PATH"
        exit 1
    fi

    if ! command -v git &> /dev/null; then
        print_error "Git is not installed or not in PATH"
        exit 1
    fi

    if ! command -v ssh &> /dev/null; then
        print_error "SSH is not installed or not in PATH"
        exit 1
    fi

    print_success "All dependencies are available"
}

# Function to check cross-compilation toolchain
check_cross_toolchain() {
    print_status "Checking cross-compilation toolchain..."

    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not installed"
        exit 1
    fi

    if ! command -v zig &> /dev/null; then
        print_error "Zig is not installed. Install with: brew install zig"
        exit 1
    fi

    if ! command -v cargo-zigbuild &> /dev/null; then
        print_error "cargo-zigbuild is not installed. Install with: cargo install cargo-zigbuild"
        exit 1
    fi

    # Check if target is installed
    if ! rustup target list --installed | grep -q "$CROSS_TARGET"; then
        print_status "Installing Rust target: $CROSS_TARGET"
        rustup target add "$CROSS_TARGET"
    fi

    print_success "Cross-compilation toolchain ready"
}

# Function to cross-compile the binary
cross_compile() {
    print_status "Cross-compiling for $CROSS_TARGET (this uses native M4 speed!)..."

    # Use SQLx offline mode for cross-compilation
    export SQLX_OFFLINE=true

    if ! cargo zigbuild --release --target "$CROSS_TARGET"; then
        print_error "Cross-compilation failed"
        exit 1
    fi

    # Verify binary was created
    local api_binary="target/${CROSS_TARGET}/release/balungpisah-core"

    if [[ ! -f "$api_binary" ]]; then
        print_error "Binary not found at: $api_binary"
        exit 1
    fi

    print_success "Cross-compilation completed: $api_binary"
}

# Function to get current git branch
get_current_branch() {
    git rev-parse --abbrev-ref HEAD
}

# Function to get git commit hash
get_commit_hash() {
    git rev-parse --short HEAD
}

# Function to determine image tag based on branch
get_image_tag() {
    local branch=$1
    local commit_hash=$2

    case $branch in
        "main")
            echo "latest"
            ;;
        "staging")
            echo "staging"
            ;;
        *)
            echo "${branch}-${commit_hash}"
            ;;
    esac
}

# Function to validate VPS configuration
validate_vps_config() {
    if [[ -z "$VPS_HOST" || -z "$VPS_USER" || -z "$COMPOSE_PATH" ]]; then
        print_error "VPS configuration is incomplete. Please check your .env.deploy.* file:"
        echo "  - VPS_HOST: $VPS_HOST"
        echo "  - VPS_USER: $VPS_USER"
        echo "  - COMPOSE_PATH: $COMPOSE_PATH"
        exit 1
    fi
}

# Function to check if user is logged in to GitHub Container Registry
check_docker_auth() {
    print_status "Checking Docker authentication..."

    if ! docker system info > /dev/null 2>&1; then
        print_error "Docker daemon is not running"
        exit 1
    fi

    # Try to login to GHCR
    if ! echo $GITHUB_TOKEN | docker login $REGISTRY -u $GITHUB_USERNAME --password-stdin 2>/dev/null; then
        print_warning "Not authenticated with GitHub Container Registry"
        print_status "Please set GITHUB_USERNAME and GITHUB_TOKEN in your .env.deploy.* file"
        print_status "Or run: docker login $REGISTRY"
        exit 1
    fi

    print_success "Docker authentication successful"
}

# Function to build Docker image (uses pre-compiled binary)
build_image() {
    local tag=$1
    local full_image_name="${IMAGE_NAME}:${tag}"

    print_status "Building Docker image: $full_image_name"

    # Build minimal image with pre-compiled binary (no QEMU emulation needed!)
    if ! docker build \
        --platform linux/amd64 \
        --tag "$full_image_name" \
        --file "$DOCKERFILE" \
        .; then
        print_error "Docker build failed"
        exit 1
    fi

    print_success "Docker image built successfully: $full_image_name"
}

# Function to push Docker image to registry
push_image() {
    local tag=$1
    local full_image_name="${IMAGE_NAME}:${tag}"

    print_status "Pushing image to registry: $full_image_name"

    if ! docker push "$full_image_name"; then
        print_error "Docker push failed"
        exit 1
    fi

    print_success "Docker image pushed to registry: $full_image_name"
}

# Function to deploy to VPS
deploy_to_vps() {
    local tag=$1
    local ssh_opts="-o StrictHostKeyChecking=no"

    if [[ -n "$VPS_KEY" ]]; then
        ssh_opts="$ssh_opts -i $VPS_KEY"
    fi

    print_status "Deploying to VPS: $VPS_HOST"

    # Create deployment commands
    local deploy_commands="
        cd $COMPOSE_PATH &&
        echo '=== Updating repository ===' &&
        git pull &&
        echo '=== Pulling Docker image ===' &&
        export IMAGE_TAG=$tag &&
        docker compose -f docker-compose.prod.yml pull &&
        echo '=== Restarting containers ===' &&
        docker compose -f docker-compose.prod.yml up -d
    "

    if ! ssh $ssh_opts "$VPS_USER@$VPS_HOST" "$deploy_commands"; then
        print_error "Deployment to VPS failed"
        exit 1
    fi

    print_success "Deployment to VPS completed"
}

# Function to verify deployment
verify_deployment() {
    local ssh_opts="-o StrictHostKeyChecking=no"

    if [[ -n "$VPS_KEY" ]]; then
        ssh_opts="$ssh_opts -i $VPS_KEY"
    fi

    print_status "Verifying deployment..."

    local verify_commands="
        cd $COMPOSE_PATH &&
        docker compose -f docker-compose.prod.yml ps &&
        docker compose -f docker-compose.prod.yml logs --tail=10
    "

    if ssh $ssh_opts "$VPS_USER@$VPS_HOST" "$verify_commands"; then
        print_success "Deployment verification completed"
    else
        print_warning "Could not verify deployment status"
    fi
}

# Function to show usage
show_usage() {
    echo "Usage: $0 <environment> [OPTIONS]"
    echo ""
    echo "Environments:"
    echo "  staging             Deploy to staging environment"
    echo "  production          Deploy to production environment"
    echo ""
    echo "Options:"
    echo "  -h, --help          Show this help message"
    echo "  -n, --no-push       Build image but don't push to registry"
    echo "  -l, --local-only    Build image only (no push, no deploy)"
    echo "  -t, --tag TAG       Use custom tag instead of branch-based tag"
    echo "  -v, --verify-only   Only verify current deployment status"
    echo ""
    echo "Configuration Files:"
    echo "  .env.deploy.staging     Staging deployment config (VPS_HOST, GITHUB_TOKEN, etc.)"
    echo "  .env.deploy.production  Production deployment config"
    echo ""
    echo "Examples:"
    echo "  $0 staging                  # Full build, push, and deploy to staging"
    echo "  $0 production               # Full build, push, and deploy to production"
    echo "  $0 staging --local-only     # Build image locally for staging"
    echo "  $0 production --tag v1.0.0  # Deploy to production with custom tag"
}

# Main deployment function
main() {
    local push_image_flag=true
    local deploy=true
    local custom_tag=""
    local verify_only=false

    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -n|--no-push)
                push_image_flag=false
                deploy=false
                shift
                ;;
            -l|--local-only)
                push_image_flag=false
                deploy=false
                shift
                ;;
            -t|--tag)
                custom_tag="$2"
                shift 2
                ;;
            -v|--verify-only)
                verify_only=true
                shift
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done

    # If verify-only mode, just check deployment status
    if [[ "$verify_only" == true ]]; then
        validate_vps_config
        verify_deployment
        exit 0
    fi

    print_status "Starting deployment process..."
    print_status "Environment: $DEPLOY_ENV"

    # Check dependencies
    check_dependencies
    check_cross_toolchain

    # Get branch and commit info
    local branch=$(get_current_branch)
    local commit_hash=$(get_commit_hash)
    local tag=${custom_tag:-$(get_image_tag "$branch" "$commit_hash")}

    print_status "Branch: $branch"
    print_status "Commit: $commit_hash"
    print_status "Tag: $tag"

    # Check if we need to push/deploy
    if [[ "$push_image_flag" == true ]]; then
        check_docker_auth
    fi

    if [[ "$deploy" == true ]]; then
        validate_vps_config
    fi

    # Confirm deployment
    if [[ "$deploy" == true ]]; then
        echo ""
        print_warning "DEPLOYMENT TARGET:"
        print_warning "  Environment: $DEPLOY_ENV"
        print_warning "  VPS Host: $VPS_HOST"
        print_warning "  Deploy Path: $COMPOSE_PATH"
        echo ""
        read -p "Continue with deployment? (y/N): " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_status "Deployment cancelled"
            exit 0
        fi
    fi

    # Cross-compile binary (fast native compilation on M4)
    cross_compile

    # Build Docker image (just copies pre-compiled binary)
    build_image "$tag"

    # Push image if requested
    if [[ "$push_image_flag" == true ]]; then
        push_image "$tag"
    fi

    # Deploy to VPS if requested
    if [[ "$deploy" == true ]]; then
        deploy_to_vps "$tag"
        verify_deployment
    fi

    print_success "Deployment process completed!"
    print_status "Environment: $DEPLOY_ENV"
    print_status "Image: ${IMAGE_NAME}:${tag}"

    if [[ "$deploy" == true ]]; then
        print_status "Deployed to: $VPS_HOST ($COMPOSE_PATH)"
    fi
}

# Run main function with all arguments
main "$@"
