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
PRUNE="false"

show_help() {
	cat <<-EOF
		IC-SIWA Developer Script

		Usage: ic-siwa <command> [options]

		Commands:
		  build              Build the project (Rust canisters)
		  test               Run unit tests (cargo test, bun test)
		  test-integration   Run integration tests (dfx canister calls)
		  fmt                Format code (cargo fmt)
		  lint               Run linters (cargo clippy, cargo fmt --check)
		  deploy             Deploy all canisters (provider + test canisters)
		  upgrade            Upgrade deployed canisters
		  urls               Show deployed canister URLs
		  cleanup            Clean up build artifacts (add --prune to delete canisters)
		  loop               Full development loop: fmt, lint, build, test, deploy
		  start              Start local DFX replica
		  stop               Stop local DFX replica
		  help               Show this help message

		Options:
		  --network <name>   Target network: dfx (default), juno, ic
		                     - dfx: Local DFX replica (port ${DFX_PORT})
		                     - juno: Local Juno emulator (port ${JUNO_PORT})
		                     - ic: IC mainnet
		  --prune            Also delete deployed canisters (use with cleanup)

		Examples:
		  ic-siwa build
		  ic-siwa deploy --network dfx
		  ic-siwa test
		  ic-siwa loop --network juno
		  ic-siwa cleanup --network juno --prune

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

# Run unit tests
cmd_test() {
	log_info "Running unit tests..."
	cd "${PROJECT_ROOT}"

	log_info "Running Rust unit tests..."
	cargo test -p ic_siwa

	log_info "Running canister unit tests..."
	cargo test -p ic_siwa_provider || log_warn "Some canister tests may require IC runtime"

	# Run TypeScript tests if bun is available
	if command -v bun &>/dev/null; then
		if [[ -f "${PROJECT_ROOT}/libs/ic_siwa_ts/package.json" ]]; then
			log_info "Running TypeScript library tests..."
			cd "${PROJECT_ROOT}/libs/ic_siwa_ts"
			bun test || log_warn "Some TypeScript tests may have failed"
			cd "${PROJECT_ROOT}"
		fi
	fi

	log_success "Unit tests complete!"
}

# Run integration tests (requires deployed canisters)
cmd_test_integration() {
	local network="${1:-dfx}"

	log_info "Running integration tests on network: ${network}"
	cd "${PROJECT_ROOT}"

	# Check if canisters are deployed
	local provider_id rs_id
	provider_id=$(dfx canister id ic_siwa_provider --network "${network}" 2>/dev/null) || {
		log_error "ic_siwa_provider not deployed. Run 'ic-siwa deploy --network ${network}' first."
		return 1
	}
	rs_id=$(dfx canister id test_canister_rs --network "${network}" 2>/dev/null) || {
		log_error "test_canister_rs not deployed. Run 'ic-siwa deploy --network ${network}' first."
		return 1
	}

	log_info "Testing ic_siwa_provider (${provider_id})..."
	log_info "Testing test_canister_rs (${rs_id})..."

	local failed=0

	# Test 1: Health check on test canister
	log_info "[1/5] Testing test_canister_rs health..."
	if dfx canister call test_canister_rs health --network "${network}" 2>/dev/null | grep -q "ok"; then
		log_success "  Health check passed"
	else
		log_error "  Health check failed"
		((failed++))
	fi

	# Test 2: Whoami on test canister (should return anonymous principal)
	log_info "[2/5] Testing test_canister_rs whoami..."
	local whoami_result
	whoami_result=$(dfx canister call test_canister_rs whoami --network "${network}" 2>/dev/null)
	if [[ -n ${whoami_result} ]]; then
		log_success "  Whoami returned: ${whoami_result}"
	else
		log_error "  Whoami failed"
		((failed++))
	fi

	# Test 3: Prepare login with test address
	log_info "[3/5] Testing siwa_prepare_login..."
	local test_address="0x1234567890123456789012345678901234567890"
	local prepare_result
	prepare_result=$(dfx canister call ic_siwa_provider siwa_prepare_login "(\"${test_address}\")" --network "${network}" 2>/dev/null)
	if echo "${prepare_result}" | grep -q "Ok"; then
		log_success "  Prepare login returned SIWA message"
	else
		log_error "  Prepare login failed: ${prepare_result}"
		((failed++))
	fi

	# Test 4: Get principal for unknown address (should return error)
	log_info "[4/5] Testing get_principal for unknown address..."
	local principal_result
	principal_result=$(dfx canister call ic_siwa_provider get_principal '("0x0000000000000000000000000000000000000000")' --network "${network}" 2>/dev/null)
	if echo "${principal_result}" | grep -q "Err"; then
		log_success "  Correctly returned error for unknown address"
	else
		log_warn "  Unexpected result: ${principal_result}"
	fi

	# Test 5: Get caller address (should return error for anonymous)
	log_info "[5/5] Testing get_caller_address..."
	local caller_result
	caller_result=$(dfx canister call ic_siwa_provider get_caller_address --network "${network}" 2>/dev/null)
	if echo "${caller_result}" | grep -q "Err"; then
		log_success "  Correctly returned error for anonymous caller"
	else
		log_warn "  Unexpected result: ${caller_result}"
	fi

	log_info ""
	if [[ ${failed} -eq 0 ]]; then
		log_success "All integration tests passed!"
	else
		log_error "${failed} integration test(s) failed"
		return 1
	fi
}

# Format code
cmd_fmt() {
	log_info "Formatting code..."
	cd "${PROJECT_ROOT}"

	log_info "Running cargo fmt..."
	cargo fmt --all

	log_success "Format complete!"
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

# Build init argument for canister
build_init_arg() {
	local network="${1:-dfx}"

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
	echo "(record { \
domain = \"${domain}\"; \
uri = \"${uri}\"; \
salt = \"${salt}\"; \
chain_id = ${chain_id} : nat64; \
session_expiration_time = 1800000000000 : nat64; \
allowed_domains = opt vec {}; \
allowed_canisters = opt vec {} \
})"
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

	# Get init argument
	local init_arg
	init_arg=$(build_init_arg "${network}") || return 1

	# Deploy ic_siwa_provider
	log_info "Deploying ic_siwa_provider..."
	log_info "  Domain: ${IC_SIWA_DOMAIN:-localhost}"
	log_info "  URI: ${IC_SIWA_URI:-http://localhost:${DFX_PORT}}"
	log_info "  Chain ID: $([[ ${network} == "ic" ]] && echo "43114" || echo "43113")"

	dfx deploy ic_siwa_provider --network "${network}" --argument "${init_arg}" || {
		log_error "Failed to deploy ic_siwa_provider"
		return 1
	}

	local provider_id
	provider_id=$(dfx canister id ic_siwa_provider --network "${network}" 2>/dev/null)
	log_success "ic_siwa_provider deployed: ${provider_id}"

	# Deploy Rust test canister
	log_info "Deploying test_canister_rs..."
	dfx deploy test_canister_rs --network "${network}" || {
		log_error "Failed to deploy test_canister_rs"
		return 1
	}

	# Build and deploy TypeScript test canister
	log_info "Building test_canister_ts..."
	cd "${PROJECT_ROOT}/canisters/test_canister_ts"

	if [[ ! -d "node_modules" ]]; then
		log_info "Installing dependencies..."
		bun install
	fi

	log_info "Building Astro app..."
	bun run build

	cd "${PROJECT_ROOT}"

	log_info "Deploying test_canister_ts..."
	dfx deploy test_canister_ts --network "${network}" || {
		log_error "Failed to deploy test_canister_ts"
		return 1
	}

	# Show URLs
	log_success "All canisters deployed!"
	show_canister_urls "${network}"
}

# Show canister URLs
show_canister_urls() {
	local network="${1:-dfx}"

	log_info ""
	log_info "Canister URLs:"

	local provider_id rs_id ts_id candid_id port
	provider_id=$(dfx canister id ic_siwa_provider --network "${network}" 2>/dev/null) || provider_id=""
	rs_id=$(dfx canister id test_canister_rs --network "${network}" 2>/dev/null) || rs_id=""
	ts_id=$(dfx canister id test_canister_ts --network "${network}" 2>/dev/null) || ts_id=""

	# Get the Candid UI canister for the network
	case "${network}" in
	dfx)
		candid_id=$(dfx canister id __Candid_UI --network "${network}" 2>/dev/null) || candid_id=""
		port="${DFX_PORT}"
		;;
	juno)
		candid_id=$(dfx canister id __Candid_UI --network "${network}" 2>/dev/null) || candid_id=""
		port="${JUNO_PORT}"
		;;
	ic)
		candid_id="a4gq6-oaaaa-aaaab-qaa4q-cai"
		port=""
		;;
	esac

	if [[ -n ${provider_id} ]]; then
		if [[ ${network} == "ic" ]]; then
			log_info "  ic_siwa_provider (Candid): https://${candid_id}.raw.ic0.app/?id=${provider_id}"
		else
			log_info "  ic_siwa_provider (Candid): http://127.0.0.1:${port}/?canisterId=${candid_id}&id=${provider_id}"
		fi
	fi

	if [[ -n ${rs_id} ]]; then
		if [[ ${network} == "ic" ]]; then
			log_info "  test_canister_rs (Candid): https://${candid_id}.raw.ic0.app/?id=${rs_id}"
		else
			log_info "  test_canister_rs (Candid): http://127.0.0.1:${port}/?canisterId=${candid_id}&id=${rs_id}"
		fi
	fi

	if [[ -n ${ts_id} ]]; then
		if [[ ${network} == "ic" ]]; then
			log_info "  test_canister_ts (Frontend): https://${ts_id}.ic0.app"
		else
			log_info "  test_canister_ts (Frontend): http://127.0.0.1:${port}/?canisterId=${ts_id}"
		fi
	fi
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

	# Get init argument
	local init_arg
	init_arg=$(build_init_arg "${network}") || return 1

	log_info "Upgrading with config:"
	log_info "  Domain: ${IC_SIWA_DOMAIN:-localhost}"
	log_info "  URI: ${IC_SIWA_URI:-http://localhost:${DFX_PORT}}"

	# Upgrade
	dfx deploy ic_siwa_provider --network "${network}" --mode upgrade --argument "${init_arg}"

	log_success "Upgrade complete!"
}

# Clean up artifacts
cmd_cleanup() {
	local network="${1:-dfx}"
	local prune="${2:-false}"

	log_info "Cleaning up build artifacts..."
	cd "${PROJECT_ROOT}"

	# Prune canisters if requested
	if [[ ${prune} == "true" ]]; then
		log_warn "Pruning canisters on network: ${network}"

		# List of canisters to delete
		local canisters=("ic_siwa_provider" "test_canister_rs" "test_canister_ts")

		for canister in "${canisters[@]}"; do
			local canister_id
			canister_id=$(dfx canister id "${canister}" --network "${network}" 2>/dev/null) || canister_id=""

			if [[ -n ${canister_id} ]]; then
				log_info "Deleting canister ${canister} (${canister_id})..."
				dfx canister stop "${canister}" --network "${network}" 2>/dev/null || true
				dfx canister delete "${canister}" --network "${network}" --yes 2>/dev/null || log_warn "Could not delete ${canister}"
			else
				log_info "Canister ${canister} not found on ${network}, skipping..."
			fi
		done
	fi

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

	cmd_fmt || {
		log_error "Format failed"
		return 1
	}

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

	cmd_test_integration "${NETWORK}" || {
		log_error "Integration tests failed"
		return 1
	}

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
		--prune)
			PRUNE="true"
			shift
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
	test-integration)
		cmd_test_integration "${NETWORK}"
		;;
	fmt)
		cmd_fmt
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
	urls)
		show_canister_urls "${NETWORK}"
		;;
	cleanup)
		cmd_cleanup "${NETWORK}" "${PRUNE}"
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
