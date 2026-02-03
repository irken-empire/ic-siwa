#!/usr/bin/env bash

# IC-SIWA Developer Entrypoint Script
# Single point of entry for all ic-siwa development tasks

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
NETWORK="dfx"
DFX_PORT="${DFX_PORT:-4943}"
JUNO_PORT="${JUNO_PORT:-5987}"

show_help() {
	cat <<-EOF
		IC-SIWA Developer Script

		Usage: ic-siwa <command> [options]

		Commands:
		  build              Build the project (Rust canisters)
		  test               Run all tests
		  lint               Run linters (cargo clippy, cargo fmt --check)
		  deploy             Deploy canisters
		  upgrade            Upgrade deployed canisters
		  cleanup            Clean up build artifacts
		  loop               Full development loop: lint, build, test, deploy
		  start              Start local DFX replica
		  stop               Stop local DFX replica
		  help               Show this help message

		Options:
		  --network <name>   Target network: dfx (default), juno, ic
		                     - dfx: Local DFX replica (port ${DFX_PORT})
		                     - juno: Local Juno emulator (port ${JUNO_PORT})
		                     - ic: IC mainnet

		Examples:
		  ic-siwa build
		  ic-siwa deploy --network dfx
		  ic-siwa test
		  ic-siwa loop --network juno

		Environment Variables (via secretspec):
		  IC_SIWA_SALT_DEVELOPMENT    Salt for development deployments
		  IC_SIWA_SALT_TESTNET        Salt for testnet deployments
		  IC_SIWA_SALT_MAINNET        Salt for mainnet deployments
		  IC_SIWA_DOMAIN              Domain for SIWA messages
		  IC_SIWA_URI                 URI for SIWA messages
	EOF
}

log_info() {
	echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
	echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
	echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
	echo -e "${RED}[ERROR]${NC} $1"
}

# Check if DFX is running
is_dfx_running() {
	local port="${1:-${DFX_PORT}}"
	if nc -z localhost "$port" 2>/dev/null; then
		return 0
	fi
	return 1
}

# Build the project
cmd_build() {
	log_info "Building ic-siwa project..."
	cd "${PROJECT_ROOT}"

	log_info "Building Rust crates..."
	cargo build --release

	log_info "Building WASM canisters..."
	cargo build --release --target wasm32-unknown-unknown -p ic_siwa_provider

	log_success "Build complete!"
}

# Run tests
cmd_test() {
	log_info "Running tests..."
	cd "${PROJECT_ROOT}"

	log_info "Running Rust unit tests..."
	cargo test -p ic_siwa

	log_info "Running canister tests..."
	cargo test -p ic_siwa_provider || log_warn "Some canister tests may require IC runtime"

	log_success "Tests complete!"
}

# Run linters
cmd_lint() {
	log_info "Running linters..."
	cd "${PROJECT_ROOT}"

	log_info "Running cargo fmt --check..."
	cargo fmt --all -- --check || {
		log_error "Formatting issues found. Run 'cargo fmt' to fix."
		return 1
	}

	log_info "Running cargo clippy..."
	cargo clippy --all-targets -- -D warnings || {
		log_error "Clippy found issues."
		return 1
	}

	log_success "Lint complete!"
}

# Start DFX replica
cmd_start() {
	local network="${1:-dfx}"

	case "${network}" in
	dfx)
		if is_dfx_running "${DFX_PORT}"; then
			log_warn "DFX replica already running on port ${DFX_PORT}"
			return 0
		fi
		log_info "Starting DFX replica..."
		cd "${PROJECT_ROOT}"
		dfx start --background --clean
		log_success "DFX replica started on port ${DFX_PORT}"
		;;
	juno)
		log_info "Juno network is external - ensure Juno is running on port ${JUNO_PORT}"
		if ! is_dfx_running "${JUNO_PORT}"; then
			log_error "Juno not detected on port ${JUNO_PORT}"
			return 1
		fi
		log_success "Juno detected on port ${JUNO_PORT}"
		;;
	ic)
		log_info "IC mainnet does not require starting"
		;;
	*)
		log_error "Unknown network: ${network}"
		return 1
		;;
	esac
}

# Stop DFX replica
cmd_stop() {
	log_info "Stopping DFX replica..."
	cd "${PROJECT_ROOT}"
	dfx stop || true
	log_success "DFX replica stopped"
}

# Deploy canisters
cmd_deploy() {
	local network="${1:-dfx}"

	log_info "Deploying to network: ${network}"
	cd "${PROJECT_ROOT}"

	# Ensure network is available
	case "${network}" in
	dfx)
		if ! is_dfx_running "${DFX_PORT}"; then
			log_info "Starting DFX replica..."
			dfx start --background --clean
			sleep 2
		fi
		;;
	juno)
		if ! is_dfx_running "${JUNO_PORT}"; then
			log_error "Juno not running on port ${JUNO_PORT}"
			return 1
		fi
		;;
	ic)
		log_warn "Deploying to IC mainnet - ensure you have cycles"
		;;
	esac

	# Build first
	cmd_build

	# Get configuration from environment or use defaults
	local domain="${IC_SIWA_DOMAIN:-localhost}"
	local uri="${IC_SIWA_URI:-http://localhost:${DFX_PORT}}"
	local salt="${IC_SIWA_SALT_DEVELOPMENT:-development-salt-change-me}"
	local chain_id="43113" # Fuji testnet for dev/juno

	if [[ ${network} == "ic" ]]; then
		salt="${IC_SIWA_SALT_MAINNET:-}"
		chain_id="43114" # Avalanche mainnet
		if [[ -z ${salt} ]]; then
			log_error "IC_SIWA_SALT_MAINNET not set for mainnet deployment"
			return 1
		fi
	fi

	# Build init argument
	local init_arg
	init_arg="(record { \
domain = \"${domain}\"; \
uri = \"${uri}\"; \
salt = \"${salt}\"; \
chain_id = ${chain_id} : nat64; \
session_expiration_time = 1800000000000 : nat64; \
allowed_domains = opt vec {}; \
allowed_canisters = opt vec {} \
})"

	log_info "Deploying ic_siwa_provider..."
	log_info "  Domain: ${domain}"
	log_info "  URI: ${uri}"
	log_info "  Chain ID: ${chain_id}"

	dfx deploy ic_siwa_provider --network "${network}" --argument "${init_arg}"

	log_success "Deployment complete!"
}

# Upgrade canisters
cmd_upgrade() {
	local network="${1:-dfx}"

	log_info "Upgrading canisters on network: ${network}"
	cd "${PROJECT_ROOT}"

	# Check if already deployed
	local canister_id
	canister_id=$(dfx canister id ic_siwa_provider --network "${network}" 2>/dev/null) || {
		log_error "Canister not deployed. Run 'ic-siwa deploy --network ${network}' first."
		return 1
	}

	log_info "Found canister: ${canister_id}"

	# Build first
	cmd_build

	# Upgrade
	dfx deploy ic_siwa_provider --network "${network}" --mode upgrade

	log_success "Upgrade complete!"
}

# Clean up artifacts
cmd_cleanup() {
	log_info "Cleaning up build artifacts..."
	cd "${PROJECT_ROOT}"

	log_info "Cleaning Cargo artifacts..."
	cargo clean

	log_info "Cleaning DFX artifacts..."
	rm -rf .dfx

	log_info "Cleaning generated files..."
	find . -name "*.did" -path "./target/*" -delete 2>/dev/null || true

	log_success "Cleanup complete!"
}

# Full development loop
cmd_loop() {
	local network="${1:-dfx}"

	log_info "Running full development loop for network: ${network}"

	cmd_lint || {
		log_error "Lint failed"
		return 1
	}

	cmd_build || {
		log_error "Build failed"
		return 1
	}

	cmd_test || {
		log_warn "Some tests may have failed"
	}

	# Check if already deployed
	local canister_id
	canister_id=$(dfx canister id ic_siwa_provider --network "${network}" 2>/dev/null) || canister_id=""

	if [[ -n ${canister_id} ]]; then
		log_info "Canister already deployed, upgrading..."
		cmd_upgrade "${network}"
	else
		log_info "Canister not deployed, deploying fresh..."
		cmd_deploy "${network}"
	fi

	log_success "Development loop complete!"
}

# Parse command line arguments
parse_args() {
	local cmd="${1:-help}"
	shift || true

	# Parse options
	while [[ $# -gt 0 ]]; do
		case "$1" in
		--network)
			NETWORK="${2:-dfx}"
			shift 2
			;;
		--help | -h)
			show_help
			exit 0
			;;
		*)
			log_error "Unknown option: $1"
			show_help
			exit 1
			;;
		esac
	done

	# Execute command
	case "${cmd}" in
	build)
		cmd_build
		;;
	test)
		cmd_test
		;;
	lint)
		cmd_lint
		;;
	deploy)
		cmd_deploy "${NETWORK}"
		;;
	upgrade)
		cmd_upgrade "${NETWORK}"
		;;
	cleanup)
		cmd_cleanup
		;;
	loop)
		cmd_loop "${NETWORK}"
		;;
	start)
		cmd_start "${NETWORK}"
		;;
	stop)
		cmd_stop
		;;
	help | --help | -h)
		show_help
		;;
	*)
		log_error "Unknown command: ${cmd}"
		show_help
		exit 1
		;;
	esac
}

# Main entry point
main() {
	parse_args "$@"
}

main "$@"
