#!/usr/bin/env bash

# IC-SIWA Developer Entrypoint Script
# Single point of entry for all ic-siwa development tasks

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Script name without extension for log file
SCRIPT_NAME="${0##*/}"
SCRIPT_NAME="${SCRIPT_NAME%.sh}"
LOG_FILE="${PROJECT_ROOT}/${SCRIPT_NAME}.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
GRAY='\033[0;90m'
NC='\033[0m' # No Color

# Log levels (numeric for comparison)
declare -A LOG_LEVELS=(
	["debug"]=0
	["info"]=1
	["success"]=1
	["warn"]=2
	["error"]=3
)

# Default values
NETWORK="dfx"
DFX_PORT="${DFX_PORT:-4943}"
JUNO_PORT="${JUNO_PORT:-5987}"
PRUNE="false"

# Logging configuration
LOG_LEVEL="${LOG_LEVEL:-warn}"            # Screen output level (debug, info, warn, error)
LOG_FILE_LEVEL="${LOG_FILE_LEVEL:-debug}" # File output level (debug, info, warn, error)

# Test wallet address (EIP-55 checksummed)
AVALANCHE_WALLET_ADDRESS="${AVALANCHE_WALLET_ADDRESS:-0xb81749C72DB5B5209098f2bd45A7a0293925DA13}"

# Expected derived principal ID for the test wallet address
# This should be deterministic for a given address + salt combination
# To find your principal: dfx canister call ic_siwa_provider get_principal '("YOUR_ADDRESS")' --network juno
# Note: This will only work after a successful login - get_principal looks up authenticated sessions
#EXPECTED_PRINCIPAL_ID="${EXPECTED_PRINCIPAL_ID:-}"
EXPECTED_PRINCIPAL_ID="h4ntr-oyuvq-xwuyv-252i6-hm3qe-hwy4w-7p5ud-kwpgd-7kwpd-y7tje-2a"

# NPM package.json files to manage (for version sync)
# Used by: cmd_version, cmd_version_force, cmd_version_check, cmd_version_sync
NPM_PACKAGES=(
	"package.json"
	"libs/ic_siwa_ts/package.json"
	"canisters/test_canister_ts/package.json"
)

# ==========================================
# Logging Functions
# ==========================================

# Log level colors and labels
declare -A LOG_COLORS=(
	["debug"]="${GRAY}"
	["info"]="${BLUE}"
	["success"]="${GREEN}"
	["warn"]="${YELLOW}"
	["error"]="${RED}"
)

declare -A LOG_LABELS=(
	["debug"]="DEBUG"
	["info"]="INFO"
	["success"]="OK"
	["warn"]="WARN"
	["error"]="ERROR"
)

# Check if a message should be logged at the given level
should_log() {
	local msg_level="$1"
	local threshold="$2"
	local msg_num="${LOG_LEVELS[$msg_level]:-1}"
	local threshold_num="${LOG_LEVELS[$threshold]:-1}"
	[[ ${msg_num} -ge ${threshold_num} ]]
}

# Write to log file (no colors)
write_log() {
	local level="$1"
	local message="$2"
	if should_log "${level}" "${LOG_FILE_LEVEL}"; then
		local timestamp
		timestamp=$(date '+%Y-%m-%d %H:%M:%S')
		echo "[${timestamp}] [${level^^}] ${message}" >>"${LOG_FILE}"
	fi
}

# Strip ANSI color codes from text
strip_colors() {
	sed 's/\x1b\[[0-9;]*m//g'
}

# Write raw output to log file (for command output, strips colors)
write_log_raw() {
	echo "$1" | strip_colors >>"${LOG_FILE}"
}

# Initialize log file
init_log() {
	# Create/truncate log file with header
	{
		echo "# IC-SIWA Log - $(date '+%Y-%m-%d %H:%M:%S')"
		echo "# Command: $0 $*"
		echo "# Log Level: screen=${LOG_LEVEL}, file=${LOG_FILE_LEVEL}"
		echo "---"
	} >"${LOG_FILE}"
}

# Unified logging function
# Usage: log <level> <message>
log() {
	local level="$1"
	local message="$2"

	write_log "${level}" "${message}"

	# Errors always shown, others check threshold
	if [[ ${level} == "error" ]] || should_log "${level}" "${LOG_LEVEL}"; then
		local color="${LOG_COLORS[$level]:-${NC}}"
		local label="${LOG_LABELS[$level]:-INFO}"
		echo -e "${color}[${label}]${NC} ${message}"
	fi
}

# Convenience wrappers
log_debug() { log "debug" "$1"; }
log_info() { log "info" "$1"; }
log_success() { log "success" "$1"; }
log_warn() { log "warn" "$1"; }
log_error() { log "error" "$1"; }

# Run a command, capturing output to log and only showing errors on screen
# Usage: run_cmd "description" command args...
run_cmd() {
	local description="$1"
	shift
	local cmd_string="$*"

	log_info "${description}"
	write_log "debug" "Running: ${cmd_string}"

	# Create temp file for output
	local tmp_out
	tmp_out=$(mktemp)

	# Run command, capture both stdout and stderr
	local exit_code=0
	if "$@" >"${tmp_out}" 2>&1; then
		exit_code=0
	else
		exit_code=$?
	fi

	# Always write output to log
	if [[ -s ${tmp_out} ]]; then
		write_log_raw "--- Command Output ---"
		write_log_raw "$(cat "${tmp_out}")"
		write_log_raw "--- End Output ---"
	fi

	# If command failed, show output on screen too
	if [[ ${exit_code} -ne 0 ]]; then
		log_error "Command failed with exit code ${exit_code}"
		# Show last 20 lines of output on screen
		if [[ -s ${tmp_out} ]]; then
			echo -e "${RED}--- Command Output (last 20 lines) ---${NC}"
			tail -20 "${tmp_out}"
			echo -e "${RED}--- End Output ---${NC}"
		fi
	fi

	rm -f "${tmp_out}"
	return ${exit_code}
}
show_help() {
	cat <<-EOF
		IC-SIWA Developer Script

		Usage: ic-siwa <command> [options]

		Commands:
		  build              Build the project (Rust canisters)
		  candid             Generate TypeScript declarations from .did files
		  test               Run unit tests (cargo test, bun test)
		  test-integration   Run integration tests (dfx canister calls)
		  fmt                Format code (cargo fmt)
		  lint               Run linters (cargo clippy, cargo fmt --check)
		  deploy             Deploy all canisters (provider + test canisters)
		  upgrade            Upgrade deployed canisters
		  urls               Show deployed canister URLs
		  cycles             Show cycles balance for all canisters and ledgers
		  cleanup            Clean up build artifacts (add --prune to delete canisters)
		  loop               Full development loop: fmt, lint, candid, build, test, deploy
		  start              Start local DFX replica
		  stop               Stop local DFX replica
		  logs               Tail canister logs in real-time (dfx canister logs --all --follow)
		  update             Update all dependencies (cargo, bun) and pin versions
		  check              Check if all required dependencies are installed
		  version            Show current version (from Cargo.toml)
		  version --bump     Bump version based on conventional commits
		  version --check    Check if all versions are in sync
		  version --sync     Sync all versions to match Cargo.toml
		  version --major    Force bump major version (x.0.0)
		  version --minor    Force bump minor version (0.x.0)
		  version --patch    Force bump patch version (0.0.x)
		  help               Show this help message

		Options:
		  --network <name>   Target network: dfx (default), juno, ic
		                     - dfx: Local DFX replica (port ${DFX_PORT})
		                     - juno: Local Juno emulator (port ${JUNO_PORT})
		                     - ic: IC mainnet
		  --prune            Also delete deployed canisters (use with cleanup)
		  --log-level <lvl>  Screen output level: debug, info (default), warn, error
		  --log-file-level   Log file level: debug (default), info, warn, error
		                     Logs written to: ${LOG_FILE}

		Examples:
		  ic-siwa build
		  ic-siwa deploy --network dfx
		  ic-siwa test
		  ic-siwa loop --network juno
		  ic-siwa cleanup --network juno --prune
		  ic-siwa update
		  ic-siwa version --bump

		Environment Variables (via secretspec):
		  IC_SIWA_SALT_DEVELOPMENT    Salt for development deployments
		  IC_SIWA_SALT_TESTNET        Salt for testnet deployments
		  IC_SIWA_SALT_MAINNET        Salt for mainnet deployments
		  IC_SIWA_DOMAIN              Domain for SIWA messages
		  IC_SIWA_URI                 URI for SIWA messages
	EOF
}

# ==========================================
# Dependency Checking
# ==========================================

# Required dependencies for each command
# Format: command:dep1,dep2,dep3
declare -A CMD_DEPS=(
	["build"]="cargo"
	["candid"]="cargo,candid-extractor,didc"
	["test"]="cargo"
	["test-integration"]="dfx"
	["fmt"]="cargo"
	["lint"]="cargo"
	["deploy"]="dfx,cargo,bun,yq"
	["upgrade"]="dfx,cargo,bun,yq"
	["loop"]="dfx,cargo,bun,yq,candid-extractor,didc"
	["start"]="dfx"
	["stop"]="dfx"
	["logs"]="dfx"
	["update"]="cargo,bun"
	["version"]="toml"
	["cleanup"]="dfx"
	["urls"]="dfx"
	["cycles"]="dfx,jq"
)

# Check if a command exists
check_cmd() {
	command -v "$1" &>/dev/null
}

# Check all dependencies for a command
check_deps() {
	local cmd="${1:-}"
	local missing=()

	# Get dependencies for this command
	local deps="${CMD_DEPS[$cmd]:-}"
	if [[ -z ${deps} ]]; then
		return 0
	fi

	# Check each dependency
	IFS=',' read -ra dep_array <<<"${deps}"
	for dep in "${dep_array[@]}"; do
		if ! check_cmd "${dep}"; then
			missing+=("${dep}")
		fi
	done

	# Report missing dependencies
	if [[ ${#missing[@]} -gt 0 ]]; then
		log_error "Missing required dependencies for '${cmd}':"
		for dep in "${missing[@]}"; do
			case "${dep}" in
			cargo)
				echo -e "  ${RED}✗${NC} cargo - Install Rust: https://rustup.rs/"
				;;
			dfx)
				echo -e "  ${RED}✗${NC} dfx - Install: sh -ci \"\$(curl -fsSL https://internetcomputer.org/install.sh)\""
				;;
			bun)
				echo -e "  ${RED}✗${NC} bun - Install: curl -fsSL https://bun.sh/install | bash"
				;;
			yq)
				echo -e "  ${RED}✗${NC} yq - Install: https://github.com/mikefarah/yq#install"
				;;
			candid-extractor)
				echo -e "  ${RED}✗${NC} candid-extractor - Install: cargo install candid-extractor"
				;;
			didc)
				echo -e "  ${RED}✗${NC} didc - Install: cargo install didc"
				;;
			toml)
				echo -e "  ${RED}✗${NC} toml - Install: cargo install toml-cli"
				;;
			convco)
				echo -e "  ${RED}✗${NC} convco - Install: cargo install convco"
				;;
			jq)
				echo -e "  ${RED}✗${NC} jq - Install via package manager (apt/brew/nix)"
				;;
			nc)
				echo -e "  ${RED}✗${NC} nc (netcat) - Install via package manager"
				;;
			*)
				echo -e "  ${RED}✗${NC} ${dep}"
				;;
			esac
		done
		echo ""
		log_info "If using devenv/nix, ensure you're in the devenv shell: devenv shell"
		return 1
	fi

	return 0
}

# Check all dependencies and show status
cmd_check_deps() {
	log_info "Checking dependencies..."
	echo ""

	local all_deps=(cargo dfx bun yq candid-extractor didc toml convco jq nc)
	local has_missing=false

	for dep in "${all_deps[@]}"; do
		if check_cmd "${dep}"; then
			local version=""
			case "${dep}" in
			cargo) version=$(cargo --version 2>/dev/null | head -1) ;;
			dfx) version=$(dfx --version 2>/dev/null | head -1) ;;
			bun) version=$(bun --version 2>/dev/null | head -1) ;;
			yq) version=$(yq --version 2>/dev/null | head -1) ;;
			toml) version="installed" ;;
			convco) version=$(convco --version 2>/dev/null | head -1) ;;
			jq) version=$(jq --version 2>/dev/null | head -1) ;;
			*) version="installed" ;;
			esac
			echo -e "  ${GREEN}✓${NC} ${dep} - ${version}"
		else
			echo -e "  ${RED}✗${NC} ${dep} - not found"
			has_missing=true
		fi
	done

	echo ""
	if [[ ${has_missing} == "true" ]]; then
		log_warn "Some optional dependencies are missing"
		log_info "Core dependencies: cargo, dfx, bun, yq"
		log_info "For Candid generation: candid-extractor, didc"
		log_info "For version management: toml, convco"
		return 1
	else
		log_success "All dependencies installed!"
	fi
}

# Check if DFX is running
is_dfx_running() {
	local port="${1:-${DFX_PORT}}"
	if nc -z localhost "$port" 2>/dev/null; then
		return 0
	fi
	return 1
}

# ==========================================
# Network Helper Functions
# ==========================================

# Get IC host URL for a network
get_ic_host() {
	local network="${1:-dfx}"
	case "${network}" in
	dfx) echo "http://127.0.0.1:${DFX_PORT}" ;;
	juno) echo "http://127.0.0.1:${JUNO_PORT}" ;;
	ic) echo "https://ic0.app" ;;
	*) echo "http://127.0.0.1:${DFX_PORT}" ;;
	esac
}

# Build TypeScript test canister
# Args: $1 = provider_canister_id, $2 = ic_host
build_ts_canister() {
	local provider_id="$1"
	local ic_host="$2"

	log_debug "  Provider Canister ID: ${provider_id}"
	log_debug "  IC Host: ${ic_host}"

	cd "${PROJECT_ROOT}/canisters/test_canister_ts"

	if [[ ! -d "node_modules" ]]; then
		run_cmd "Installing dependencies..." bun install || return 1
	fi

	run_cmd "Building Astro app..." \
		env PUBLIC_SIWA_PROVIDER_CANISTER_ID="${provider_id}" \
		PUBLIC_IC_HOST="${ic_host}" \
		bun run build || return 1

	cd "${PROJECT_ROOT}"
}

# Version management
# Source of truth: Cargo.toml [workspace.package.version]
cmd_version() {
	local bump="${1:-false}"
	cd "${PROJECT_ROOT}"

	# Check for required tools
	if ! command -v convco &>/dev/null; then
		log_error "convco not found. Install via: cargo install convco"
		return 1
	fi

	# Get current version from Cargo.toml (source of truth)
	local current_version
	current_version=$(toml get Cargo.toml workspace.package.version --raw)

	if [[ ${bump} == "false" ]]; then
		# Just show current version
		echo "${current_version}"
		return 0
	fi

	# Ensure we have the latest tags pulled down.
	git fetch --tags >/dev/null 2>&1 || {
		log_warn "Failed to fetch tags, convco may not work correctly."
	}

	# Get next version from convco based on conventional commits
	local next_version
	next_version=$(convco version --bump 2>/dev/null || echo "")

	if [[ -z ${next_version} ]]; then
		log_warn "No version bump needed based on commits, or no previous tag found"
		log_info "Current version: ${current_version}"
		log_info "Use --major, --minor, or --patch to force a bump"
		return 0
	fi

	# Remove 'v' prefix if present (convco may add it)
	next_version="${next_version#v}"

	if [[ ${current_version} == "${next_version}" ]]; then
		log_info "Version already at ${current_version}, no bump needed"
		return 0
	fi

	log_info "Bumping version: ${current_version} -> ${next_version}"

	# Update Cargo.toml (workspace version - source of truth)
	log_info "Updating Cargo.toml..."
	toml set Cargo.toml workspace.package.version "${next_version}" >Cargo.toml.tmp
	mv Cargo.toml.tmp Cargo.toml

	# Update all npm package.json files
	for npm_package in "${NPM_PACKAGES[@]}"; do
		local full_path="${PROJECT_ROOT}/${npm_package}"
		if [[ -f ${full_path} ]]; then
			log_info "Updating ${npm_package}..."
			jq --arg v "${next_version}" '.version = $v' "${full_path}" >"${full_path}.tmp"
			mv "${full_path}.tmp" "${full_path}"
		fi
	done

	# Update Cargo.lock by running cargo check
	log_info "Updating Cargo.lock..."
	cargo check --quiet 2>/dev/null || true

	log_success "Version bumped to ${next_version}"
	log_info ""
	log_info "Files updated:"
	log_info "  - Cargo.toml (workspace.package.version)"
	for npm_package in "${NPM_PACKAGES[@]}"; do
		log_info "  - ${npm_package}"
	done
	log_info "  - Cargo.lock"
	log_info ""
	log_info "Next steps:"
	log_info "  1. Review changes: git diff"
	log_info "  2. Commit: git commit -am 'chore(release): ${next_version}'"
	log_info "  3. Tag: git tag v${next_version}"
	log_info "  4. Push: git push && git push --tags"
}

# Force version bump (major/minor/patch)
cmd_version_force() {
	local bump_type="${1:-patch}"
	cd "${PROJECT_ROOT}"

	local current_version
	current_version=$(toml get Cargo.toml workspace.package.version --raw)

	# Parse version components
	IFS='.' read -r major minor patch <<<"${current_version}"

	case "${bump_type}" in
	major)
		major=$((major + 1))
		minor=0
		patch=0
		;;
	minor)
		minor=$((minor + 1))
		patch=0
		;;
	patch)
		patch=$((patch + 1))
		;;
	*)
		log_error "Invalid bump type: ${bump_type}. Use major, minor, or patch"
		return 1
		;;
	esac

	local next_version="${major}.${minor}.${patch}"
	log_info "Force bumping version: ${current_version} -> ${next_version} (${bump_type})"

	# Update Cargo.toml
	toml set Cargo.toml workspace.package.version "${next_version}" >Cargo.toml.tmp
	mv Cargo.toml.tmp Cargo.toml

	# Update all npm package.json files
	for npm_package in "${NPM_PACKAGES[@]}"; do
		local full_path="${PROJECT_ROOT}/${npm_package}"
		if [[ -f ${full_path} ]]; then
			jq --arg v "${next_version}" '.version = $v' "${full_path}" >"${full_path}.tmp"
			mv "${full_path}.tmp" "${full_path}"
		fi
	done

	cargo check --quiet 2>/dev/null || true

	log_success "Version bumped to ${next_version}"
}

# Check if versions are in sync
cmd_version_check() {
	cd "${PROJECT_ROOT}"

	local cargo_version
	cargo_version=$(toml get Cargo.toml workspace.package.version --raw)

	local has_error=false

	log_info "Version check (source: Cargo.toml workspace.package.version)"
	echo "  Cargo.toml (workspace):        ${cargo_version}"

	for npm_package in "${NPM_PACKAGES[@]}"; do
		local full_path="${PROJECT_ROOT}/${npm_package}"
		if [[ -f ${full_path} ]]; then
			local npm_version
			npm_version=$(jq -r '.version' "${full_path}")
			if [[ ${cargo_version} == "${npm_version}" ]]; then
				echo "  ${npm_package}: ${npm_version} ✓"
			else
				echo "  ${npm_package}: ${npm_version} ✗ (mismatch!)"
				has_error=true
			fi
		fi
	done

	if [[ ${has_error} == "true" ]]; then
		log_error "Version mismatch detected! Run 'ic-siwa version --sync' to fix."
		return 1
	fi

	log_success "All versions in sync: ${cargo_version}"
}

# Sync versions to match Cargo.toml
cmd_version_sync() {
	cd "${PROJECT_ROOT}"

	local cargo_version
	cargo_version=$(toml get Cargo.toml workspace.package.version --raw)

	log_info "Syncing all versions to ${cargo_version}..."

	for npm_package in "${NPM_PACKAGES[@]}"; do
		local full_path="${PROJECT_ROOT}/${npm_package}"
		if [[ -f ${full_path} ]]; then
			jq --arg v "${cargo_version}" '.version = $v' "${full_path}" >"${full_path}.tmp"
			mv "${full_path}.tmp" "${full_path}"
			log_info "Updated ${npm_package}"
		fi
	done

	log_success "Versions synced to ${cargo_version}"
}

# Update all dependencies
cmd_update() {
	cd "${PROJECT_ROOT}"
	log_info "Updating all dependencies..."

	# All package.json directories to update
	local -a npm_dirs=(
		"."
		"libs/ic_siwa_ts"
		"canisters/test_canister_ts"
	)

	# Update Cargo dependencies
	log_info "Updating Cargo dependencies..."
	cargo update
	log_success "Cargo dependencies updated"

	# Update bun packages to latest versions with exact versions (no ^ or ~ prefixes)
	for npm_dir in "${npm_dirs[@]}"; do
		local full_path="${PROJECT_ROOT}/${npm_dir}"
		if [[ -d ${full_path} ]]; then
			log_info "Updating bun packages in ${npm_dir}..."
			(
				cd "${full_path}"
				# --latest updates to newest versions regardless of current constraints
				# --save-exact ensures no ^ or ~ prefixes are added
				bun update --latest --save-exact
			)
			log_success "Updated ${npm_dir}"
		fi
	done

	log_success "All dependencies updated!"
	log_info ""
	log_info "Next steps:"
	log_info "  1. Review changes: git diff"
	log_info "  2. Test: ic-siwa test"
	log_info "  3. Commit: git commit -am 'chore(deps): update dependencies'"
}

# Build the project
cmd_build() {
	log_info "Building ic-siwa project..."
	cd "${PROJECT_ROOT}"

	run_cmd "Building Rust crates..." cargo build --release || return 1

	run_cmd "Building WASM canisters..." cargo build --release --target wasm32-unknown-unknown -p ic_siwa_provider || return 1

	# Build TypeScript library (required before test_canister_ts can use it)
	if [[ -f "${PROJECT_ROOT}/libs/ic_siwa_ts/package.json" ]]; then
		cd "${PROJECT_ROOT}/libs/ic_siwa_ts"
		if [[ ! -d "node_modules" ]]; then
			run_cmd "Installing ic_siwa_ts dependencies..." bun install || return 1
		fi
		run_cmd "Building ic_siwa_ts library..." bun run build || return 1
		cd "${PROJECT_ROOT}"
	fi

	log_success "Build complete!"
}

# Generate TypeScript declarations from Candid files
cmd_candid() {
	log_info "Generating Candid interface and TypeScript declarations..."
	cd "${PROJECT_ROOT}"

	# Check if candid-extractor is available
	if ! command -v candid-extractor &>/dev/null; then
		log_error "candid-extractor not found. Install via: cargo install candid-extractor"
		return 1
	fi

	# Check if didc is available
	if ! command -v didc &>/dev/null; then
		log_error "didc not found. Install the Candid compiler for TypeScript generation."
		log_info "  Install via: cargo install didc"
		log_info "  Or add to devenv/nix: didc"
		return 1
	fi

	local wasm_file="${PROJECT_ROOT}/target/wasm32-unknown-unknown/release/ic_siwa_provider.wasm"
	local did_file="${PROJECT_ROOT}/canisters/ic_siwa_provider/ic_siwa_provider.did"
	local ts_output_dir="${PROJECT_ROOT}/libs/ic_siwa_ts/src/candid"
	local ts_file="${ts_output_dir}/ic_siwa_provider.ts"

	# Build the WASM if it doesn't exist or is older than source
	if [[ ! -f ${wasm_file} ]]; then
		log_info "WASM not found, building..."
		cargo build --target wasm32-unknown-unknown --release -p ic_siwa_provider || {
			log_error "Failed to build WASM"
			return 1
		}
	fi

	# Step 1: Extract .did from WASM using candid-extractor
	log_info "Extracting Candid interface from WASM..."
	candid-extractor "${wasm_file}" >"${did_file}.extracted" || {
		log_error "Failed to extract Candid from WASM"
		return 1
	}

	# Replace the .did file with extracted version
	mv "${did_file}.extracted" "${did_file}"
	log_success "Candid interface extracted: ${did_file}"

	# Create output directory
	mkdir -p "${ts_output_dir}"

	# Step 2: Generate TypeScript from .did using didc
	log_info "Generating TypeScript module..."
	{
		echo '// @ts-nocheck'
		echo '/**'
		echo ' * Auto-generated Candid bindings for ic_siwa_provider canister'
		echo ' * Generated from: canisters/ic_siwa_provider/ic_siwa_provider.did'
		echo ' * DO NOT EDIT MANUALLY - regenerate with: ic-siwa candid'
		echo ' */'
		echo ''
		echo '/* eslint-disable @typescript-eslint/no-explicit-any */'
		echo ''
		# Get TypeScript types, but fix the IDL import to be a value import (not type)
		# and skip the declare lines at the end - we provide real implementations
		didc bind "${did_file}" -t ts | grep -v "^export declare" | sed 's/import type { IDL }/import { IDL }/'
		echo ''
		# Get IDL factory from JS output, add TypeScript typing
		didc bind "${did_file}" -t js | sed 's/{ IDL }/{ IDL }: { IDL: any }/g'
	} >"${ts_file}" || {
		log_error "Failed to generate TypeScript module"
		return 1
	}

	# Update index.ts to re-export
	cat <<-EOF >"${ts_output_dir}/index.ts"
		/**
		 * Candid TypeScript declarations
		 * Auto-generated - DO NOT EDIT MANUALLY
		 */
		export * from './ic_siwa_provider';
	EOF

	log_success "TypeScript module generated: ${ts_file}"
}

# Run unit tests
cmd_test() {
	log_info "Running unit tests..."
	cd "${PROJECT_ROOT}"

	run_cmd "Running Rust unit tests..." cargo test -p ic_siwa || return 1

	run_cmd "Running canister unit tests..." cargo test -p ic_siwa_provider || log_warn "Some canister tests may require IC runtime"

	# Run TypeScript tests if bun is available
	if command -v bun &>/dev/null; then
		if [[ -f "${PROJECT_ROOT}/libs/ic_siwa_ts/package.json" ]]; then
			cd "${PROJECT_ROOT}/libs/ic_siwa_ts"
			run_cmd "Running TypeScript library tests..." bun test || log_warn "Some TypeScript tests may have failed"
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
	local test_num=0
	local total_tests=11

	# Test 1: Health check on test canister
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing test_canister_rs health..."
	if dfx canister call test_canister_rs health --network "${network}" 2>/dev/null | grep -q "ok"; then
		log_success "  Health check passed"
	else
		log_error "  Health check failed"
		failed=$((failed + 1))
	fi

	# Test 2: Whoami on test canister (should return anonymous principal)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing test_canister_rs whoami..."
	local whoami_result
	whoami_result=$(dfx canister call test_canister_rs whoami --network "${network}" 2>/dev/null)
	if [[ -n ${whoami_result} ]]; then
		log_success "  Whoami returned: ${whoami_result}"
	else
		log_error "  Whoami failed"
		failed=$((failed + 1))
	fi

	# Test 3: Prepare login with test address
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing siwa_prepare_login..."
	local test_address="0x1234567890123456789012345678901234567890"
	local prepare_result
	prepare_result=$(dfx canister call ic_siwa_provider siwa_prepare_login "(\"${test_address}\")" --network "${network}" 2>/dev/null)
	if echo "${prepare_result}" | grep -q "Ok"; then
		log_success "  Prepare login returned SIWA message"
	else
		log_error "  Prepare login failed: ${prepare_result}"
		failed=$((failed + 1))
	fi

	# Test 4: Get principal for unknown address (should return error)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing get_principal for unknown address..."
	local principal_result
	principal_result=$(dfx canister call ic_siwa_provider get_principal '("0x0000000000000000000000000000000000000000")' --network "${network}" 2>/dev/null)
	if echo "${principal_result}" | grep -q "Err"; then
		log_success "  Correctly returned error for unknown address"
	else
		log_warn "  Unexpected result: ${principal_result}"
	fi

	# Test 5: Get caller address (should return error for anonymous)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing get_caller_address..."
	local caller_result
	caller_result=$(dfx canister call ic_siwa_provider get_caller_address --network "${network}" 2>/dev/null)
	if echo "${caller_result}" | grep -q "Err"; then
		log_success "  Correctly returned error for anonymous caller"
	else
		log_warn "  Unexpected result: ${caller_result}"
	fi

	# ==========================================
	# Security Tests
	# ==========================================
	log_info ""
	log_info "Running security tests..."

	# Test 6: Debug endpoint - should work in dev/testnet, fail in mainnet
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing debug_info endpoint..."
	local debug_result
	debug_result=$(dfx canister call ic_siwa_provider debug_info --network "${network}" 2>/dev/null)
	if [[ ${network} == "ic" ]]; then
		# Mainnet: debug should be disabled
		if echo "${debug_result}" | grep -q "Err"; then
			log_success "  Debug endpoint correctly disabled on mainnet"
		else
			log_error "  Debug endpoint should be disabled on mainnet!"
			failed=$((failed + 1))
		fi
	else
		# Dev/Testnet: debug should be enabled
		if echo "${debug_result}" | grep -q "Ok"; then
			log_success "  Debug endpoint available (dev/testnet mode)"
			# Extract and display some debug info
			if echo "${debug_result}" | grep -q "chain_id"; then
				log_info "    $(echo "${debug_result}" | grep -o 'chain_id = [0-9_]*')"
			fi
		else
			log_warn "  Debug endpoint not available: ${debug_result}"
		fi
	fi

	# Test 7: Invalid address format rejection
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing invalid address rejection..."
	local invalid_addresses=(
		"invalid"
		"0x123"
		"5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed"
		"0xGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG"
	)
	local invalid_blocked=0

	for addr in "${invalid_addresses[@]}"; do
		local invalid_result
		invalid_result=$(dfx canister call ic_siwa_provider siwa_prepare_login "(\"${addr}\")" --network "${network}" 2>/dev/null)
		if echo "${invalid_result}" | grep -q "Err"; then
			invalid_blocked=$((invalid_blocked + 1))
		fi
	done

	if [[ ${invalid_blocked} -eq ${#invalid_addresses[@]} ]]; then
		log_success "  All ${invalid_blocked} invalid addresses correctly rejected"
	else
		log_error "  Only ${invalid_blocked}/${#invalid_addresses[@]} invalid addresses rejected"
		failed=$((failed + 1))
	fi

	# Test 8: Session expiration check (delegation without valid session)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing delegation without session..."
	# siwa_get_delegation expects (text, blob, nat64) - pass a fake session key blob and expiration
	local delegation_result
	delegation_result=$(dfx canister call ic_siwa_provider siwa_get_delegation '("0xNoSession0000000000000000000000000000001", blob "\00\01\02\03\04\05\06\07\08\09\0a\0b\0c\0d\0e\0f", 3600000000000 : nat64)' --network "${network}" 2>/dev/null)
	if echo "${delegation_result}" | grep -qi "Err\|No authenticated session"; then
		log_success "  Delegation correctly requires authenticated session"
	else
		log_error "  Delegation should fail without valid session"
		log_error "    Got: ${delegation_result}"
		failed=$((failed + 1))
	fi

	# Test 9: Verify allowed_domains is being checked (via debug endpoint if available)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing security configuration..."
	if echo "${debug_result}" | grep -q "Ok"; then
		# Check if security settings are present
		if echo "${debug_result}" | grep -q "allowed_domains\|rate_limits"; then
			log_success "  Security configuration active"
			# Show configured domains if any
			if echo "${debug_result}" | grep -q 'allowed_domains = vec {[^}]*"'; then
				local domains
				domains=$(echo "${debug_result}" | grep -o 'allowed_domains = vec {[^}]*}' | head -1)
				log_info "    ${domains}"
			fi
		else
			log_warn "  Security configuration not visible in debug output"
		fi
	else
		log_info "  Skipped (debug endpoint not available)"
	fi

	# Test 10: Principal derivation consistency (requires prior successful login)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing principal derivation consistency..."
	if [[ -n ${EXPECTED_PRINCIPAL_ID} ]]; then
		local derived_principal
		derived_principal=$(dfx canister call ic_siwa_provider get_principal "(\"${AVALANCHE_WALLET_ADDRESS}\")" --network "${network}" 2>/dev/null)
		if echo "${derived_principal}" | grep -q "Ok"; then
			# Extract the principal from the response
			local actual_principal
			actual_principal=$(echo "${derived_principal}" | grep -o 'principal "[^"]*"' | sed 's/principal "//;s/"//')
			if [[ ${actual_principal} == "${EXPECTED_PRINCIPAL_ID}" ]]; then
				log_success "  Principal derivation is consistent: ${actual_principal}"
			else
				log_error "  Principal mismatch!"
				log_error "    Expected: ${EXPECTED_PRINCIPAL_ID}"
				log_error "    Got:      ${actual_principal}"
				failed=$((failed + 1))
			fi
		else
			log_info "  Skipped (no authenticated session found)"
			log_info "    1. Login via browser UI with wallet: ${AVALANCHE_WALLET_ADDRESS}"
			log_info "    2. Run: dfx canister call ic_siwa_provider get_principal '(\"${AVALANCHE_WALLET_ADDRESS}\")' --network ${network}"
			log_info "    3. Set EXPECTED_PRINCIPAL_ID to the returned principal"
		fi
	else
		log_warn "  Skipped (EXPECTED_PRINCIPAL_ID not set)"
		log_warn "    To enable this test:"
		log_warn "    1. Login via browser UI with wallet: ${AVALANCHE_WALLET_ADDRESS}"
		log_warn "    2. Run: dfx canister call ic_siwa_provider get_principal '(\"${AVALANCHE_WALLET_ADDRESS}\")' --network ${network}"
		log_warn '    3. Export: EXPECTED_PRINCIPAL_ID="<principal-from-step-2>"'
		log_warn "    4. Re-run: ic-siwa test-integration --network ${network}"
		log_warn "  NOTE: You can't run this test with 'loop' as it resets the login counter. You must use 'test-integration' directly."
	fi

	# Test 11: Rate limiting - rapid requests should eventually be blocked
	# NOTE: This test MUST be last - it triggers rate limits that block requests for 60 seconds
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing rate limiting..."

	# Read rate limit config from the same config file used for deployment
	local config_file
	case "${network}" in
	dfx | juno) config_file="${PROJECT_ROOT}/config/development.yaml" ;;
	testnet) config_file="${PROJECT_ROOT}/config/testnet.yaml" ;;
	ic) config_file="${PROJECT_ROOT}/config/mainnet.yaml" ;;
	*) config_file="${PROJECT_ROOT}/config/development.yaml" ;;
	esac

	local max_per_address=5
	local window_seconds=60
	if [[ -f ${config_file} ]] && command -v yq &>/dev/null; then
		max_per_address=$(yq -r '.security.rate_limits.max_logins_per_address // 5' "${config_file}")
		window_seconds=$(yq -r '.security.rate_limits.window_seconds // 60' "${config_file}")
	fi

	log_info "  Config: max_logins_per_address=${max_per_address}, window=${window_seconds}s"

	# Use the configured test wallet address
	local rate_test_address="${AVALANCHE_WALLET_ADDRESS}"
	local rate_blocked=false
	local rate_count=0
	local last_error=""

	# Make requests to trigger per-address rate limit
	# We need max_per_address + 1 requests to trigger the limit
	local requests_needed=$((max_per_address + 1))
	log_info "  Sending ${requests_needed} requests to trigger per-address limit..."

	for ((i = 1; i <= requests_needed; i++)); do
		local rate_result
		rate_result=$(dfx canister call ic_siwa_provider siwa_prepare_login "(\"${rate_test_address}\")" --network "${network}" 2>/dev/null)
		if echo "${rate_result}" | grep -qi "rate.*limit\|too many\|Rate limit"; then
			rate_blocked=true
			rate_count=$i
			break
		fi
		# Check if it's an error (but not rate limit)
		if echo "${rate_result}" | grep -q "Err"; then
			last_error="${rate_result}"
		fi
	done

	if [[ ${rate_blocked} == "true" ]]; then
		log_success "  Rate limiting triggered after ${rate_count} requests"

		# Warn user about the cooldown period
		log_warn ""
		log_warn "  Rate limit is now active for ${window_seconds} seconds!"
		log_warn "  Login attempts will be blocked until: $(date -d "+${window_seconds} seconds" '+%H:%M:%S' 2>/dev/null || date -v+"${window_seconds}"S '+%H:%M:%S' 2>/dev/null || echo "~${window_seconds}s from now")"
		log_warn ""

		# Wait for the rate limit window to expire
		log_info "  Waiting ${window_seconds}s for rate limit window to expire..."
		local remaining=${window_seconds}
		while [[ ${remaining} -gt 0 ]]; do
			# Show countdown every 10 seconds or for last 5 seconds
			if [[ $((remaining % 10)) -eq 0 ]] || [[ ${remaining} -le 5 ]]; then
				printf "\r  Cooldown: %ds remaining...   " "${remaining}"
			fi
			sleep 1
			remaining=$((remaining - 1))
		done
		printf "\r  Cooldown complete!              \n"
		log_success "  Rate limit window expired - logins are now allowed"
	else
		log_warn "  Rate limiting not triggered after ${requests_needed} requests"
		if [[ -n ${last_error} ]]; then
			log_warn "    Last response: ${last_error:0:100}..."
		fi
		log_info "  This may indicate rate limits are set higher than expected"
	fi

	# Summary
	log_info ""
	if [[ ${failed} -eq 0 ]]; then
		log_success "All ${total_tests} integration tests passed!"
	else
		log_error "${failed} integration test(s) failed"
		return 1
	fi
}

# Format code
cmd_fmt() {
	log_info "Formatting code..."
	cd "${PROJECT_ROOT}"

	run_cmd "Running cargo fmt..." cargo fmt --all || return 1

	log_success "Format complete!"
}

# Run linters
cmd_lint() {
	log_info "Running linters..."
	cd "${PROJECT_ROOT}"

	run_cmd "Running cargo fmt --check..." cargo fmt --all -- --check || {
		log_error "Formatting issues found. Run 'cargo fmt' to fix."
		return 1
	}

	run_cmd "Running cargo clippy..." cargo clippy --all-targets -- -D warnings || {
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
		log_warn "You cannot start Juno from this project"
		log_info "Please start Juno from your Juno project, then run: ic-siwa deploy --network juno"
		log_info ""
		if is_dfx_running "${JUNO_PORT}"; then
			log_success "Juno is running on port ${JUNO_PORT} - ready to deploy"
		else
			log_error "Juno not detected on port ${JUNO_PORT}"
			return 1
		fi
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
	local network="${1:-local}"
	cd "${PROJECT_ROOT}"

	case "${network}" in
	juno)
		log_warn "Juno is managed separately - ic-siwa does not control Juno's lifecycle"
		log_info "Canisters deployed to Juno will persist until Juno is restarted"
		log_info ""
		log_info "Options:"
		log_info "  - To delete ic-siwa canisters from Juno: ic-siwa cleanup --prune --network juno"
		log_info "  - To stop Juno entirely: run 'juno stop' from your Juno project"
		;;
	ic)
		log_info "Network 'ic' is the Internet Computer mainnet - nothing to stop locally"
		log_info "Use 'cleanup --prune --network ic' to delete canisters (DANGER: permanent!)"
		;;
	*)
		log_info "Stopping DFX local replica..."
		dfx stop || true
		log_success "DFX replica stopped"
		;;
	esac
}

# Tail canister logs in real-time
cmd_logs() {
	local network="${1:-dfx}"
	cd "${PROJECT_ROOT}"

	case "${network}" in
	juno)
		log_warn "Juno satellite logs are not directly accessible via dfx"
		log_info "For Juno, check the Juno console or use ic_cdk::println! with a debug endpoint"
		return 1
		;;
	ic)
		log_warn "Mainnet canister logs require the IC management canister"
		log_info "Use: dfx canister logs ic_siwa_provider --network ic"
		log_info "Note: ic_cdk::println! output may not be available on mainnet"
		return 1
		;;
	*)
		log_info "Tailing canister logs (Ctrl+C to stop)..."
		dfx canister logs --all --follow
		;;
	esac
}

# Build init argument for canister
build_init_arg() {
	local network="${1:-dfx}"

	# Map network to config file
	local config_file
	case "${network}" in
	dfx | juno) config_file="${PROJECT_ROOT}/config/development.yaml" ;;
	testnet) config_file="${PROJECT_ROOT}/config/testnet.yaml" ;;
	ic) config_file="${PROJECT_ROOT}/config/mainnet.yaml" ;;
	*) config_file="${PROJECT_ROOT}/config/development.yaml" ;;
	esac

	if [[ ! -f ${config_file} ]]; then
		log_error "Config file not found: ${config_file}"
		return 1
	fi

	# Log to stderr so it doesn't get captured in the return value
	echo -e "${BLUE}[INFO]${NC} Reading config from: ${config_file}" >&2

	# Get configuration from environment or use defaults
	local domain="${IC_SIWA_DOMAIN:-localhost}"
	local uri="${IC_SIWA_URI:-http://localhost:${DFX_PORT}}"
	local salt="${IC_SIWA_SALT_DEVELOPMENT:-development-salt-change-me}"

	# Read values from config file
	local chain_id
	chain_id=$(yq -r '.avalanche.chain_id' "${config_file}")

	local session_exp_seconds
	session_exp_seconds=$(yq -r '.security.session_expiration_seconds // 1800' "${config_file}")
	local session_exp_ns=$((session_exp_seconds * 1000000000))

	# Read allowed_domains as Candid vec
	local allowed_domains
	allowed_domains=$(yq -r '.security.allowed_domains | map("\"" + . + "\"") | join("; ")' "${config_file}")

	# Read allowed_canisters as Candid vec
	local allowed_canisters
	allowed_canisters=$(yq -r '.security.allowed_canisters | map("principal \"" + . + "\"") | join("; ")' "${config_file}")

	# Read delegation_targets as Candid vec
	local delegation_targets
	delegation_targets=$(yq -r '.security.delegation_targets | map("principal \"" + . + "\"") | join("; ")' "${config_file}")

	# Read rate limit settings with defaults
	local rate_limit_per_address
	rate_limit_per_address=$(yq -r '.security.rate_limits.max_logins_per_address // 10' "${config_file}")
	local rate_limit_total
	rate_limit_total=$(yq -r '.security.rate_limits.max_logins_total // 1000' "${config_file}")
	local rate_limit_window
	rate_limit_window=$(yq -r '.security.rate_limits.window_seconds // 3600' "${config_file}")

	# Read debug flag (defaults to false)
	local debug
	debug=$(yq -r '.debug // false' "${config_file}")

	if [[ ${network} == "ic" ]]; then
		salt="${IC_SIWA_SALT_MAINNET:-}"
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
session_expiration_time = ${session_exp_ns} : nat64; \
allowed_domains = opt vec { ${allowed_domains} }; \
allowed_canisters = opt vec { ${allowed_canisters} }; \
delegation_targets = opt vec { ${delegation_targets} }; \
rate_limits = opt record { \
max_logins_per_address = ${rate_limit_per_address} : nat32; \
max_logins_total = ${rate_limit_total} : nat32; \
window_seconds = ${rate_limit_window} : nat64; \
}; \
debug = opt ${debug}; \
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
	log_debug "  Domain: ${IC_SIWA_DOMAIN:-localhost}"
	log_debug "  URI: ${IC_SIWA_URI:-http://localhost:${DFX_PORT}}"
	log_debug "  Chain ID: $([[ ${network} == "ic" ]] && echo "43114" || echo "43113")"

	run_cmd "Deploying ic_siwa_provider..." dfx deploy ic_siwa_provider --network "${network}" --argument "${init_arg}" --yes || {
		log_error "Failed to deploy ic_siwa_provider"
		return 1
	}

	local provider_id
	provider_id=$(dfx canister id ic_siwa_provider --network "${network}" 2>/dev/null)
	log_success "ic_siwa_provider deployed: ${provider_id}"

	# Deploy Rust test canister
	run_cmd "Deploying test_canister_rs..." dfx deploy test_canister_rs --network "${network}" --yes || {
		log_error "Failed to deploy test_canister_rs"
		return 1
	}

	# Build and deploy TypeScript test canister
	local ic_host
	ic_host=$(get_ic_host "${network}")
	build_ts_canister "${provider_id}" "${ic_host}" || return 1

	run_cmd "Deploying test_canister_ts..." dfx deploy test_canister_ts --network "${network}" --yes || {
		log_error "Failed to deploy test_canister_ts"
		return 1
	}

	log_success "All canisters deployed!"

	# Remind about direnv if .env was updated
	if [[ -f "${PROJECT_ROOT}/.env" ]]; then
		log_info ""
		log_warn "If using direnv, run 'direnv reload' to pick up new .env values"
	fi
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

	log_info "Found ic_siwa_provider: ${canister_id}"

	# Build Rust canisters first
	cmd_build

	# Get init argument
	local init_arg
	init_arg=$(build_init_arg "${network}") || return 1

	log_info "Upgrading with config:"
	log_info "  Domain: ${IC_SIWA_DOMAIN:-localhost}"
	log_info "  URI: ${IC_SIWA_URI:-http://localhost:${DFX_PORT}}"

	# Upgrade ic_siwa_provider
	# Use --upgrade-unchanged to force upgrade even if WASM hash is the same
	# This ensures new init_args (config) are applied
	run_cmd "Upgrading ic_siwa_provider..." dfx deploy ic_siwa_provider --network "${network}" --mode upgrade --upgrade-unchanged --argument "${init_arg}" --yes || return 1

	# Upgrade test_canister_rs
	run_cmd "Upgrading test_canister_rs..." dfx deploy test_canister_rs --network "${network}" --mode upgrade --yes || return 1

	# Rebuild and upgrade test_canister_ts
	local ic_host
	ic_host=$(get_ic_host "${network}")
	build_ts_canister "${canister_id}" "${ic_host}" || return 1

	run_cmd "Upgrading test_canister_ts..." dfx deploy test_canister_ts --network "${network}" --mode upgrade --yes || return 1

	log_success "All canisters upgraded!"

	# Remind about direnv if .env was updated
	if [[ -f "${PROJECT_ROOT}/.env" ]]; then
		log_info ""
		log_warn "If using direnv, run 'direnv reload' to pick up new .env values"
	fi
}

# Show cycles balance for all canisters
cmd_cycles() {
	cd "${PROJECT_ROOT}"

	local canister_ids_file="${PROJECT_ROOT}/canister_ids.json"
	local environments=("testnet" "mainnet")

	# Network to identity mapping
	declare -A NETWORK_IDENTITIES=(
		["testnet"]="ic-siwa-testnet"
		["mainnet"]="ic-siwa-mainnet"
	)

	local current_identity
	current_identity=$(dfx identity whoami)

	# Cleanup function to remove temporary canister_ids.json and restore identity
	cleanup_cycles() {
		if [[ -f ${canister_ids_file} ]]; then
			rm -f "${canister_ids_file}"
			log_debug "Cleaned up temporary canister_ids.json"
		fi
		dfx identity use "${current_identity}" >/dev/null 2>&1
	}

	# Set trap to ensure cleanup runs on exit, error, or interrupt
	trap cleanup_cycles EXIT ERR INT TERM

	log_info "Checking cycles balances..."
	echo ""

	for env in "${environments[@]}"; do
		local env_file="${PROJECT_ROOT}/canister_ids.${env}.json"
		local identity="${NETWORK_IDENTITIES[$env]}"

		# Check if environment file exists
		if [[ ! -f ${env_file} ]]; then
			log_warn "canister_ids.${env}.json not found, skipping ${env}"
			continue
		fi

		# Check if identity exists
		if ! dfx identity list 2>/dev/null | grep -q "^${identity}"; then
			log_warn "Identity '${identity}' not found, skipping ${env}"
			continue
		fi

		# Copy environment-specific file to canister_ids.json
		cp "${env_file}" "${canister_ids_file}" || {
			log_error "Failed to copy canister_ids.${env}.json"
			return 1
		}

		# Switch to the appropriate identity
		dfx identity use "${identity}" >/dev/null 2>&1 || {
			log_error "Failed to switch to identity '${identity}'"
			return 1
		}

		echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
		echo -e "${BLUE}Environment: ${env}${NC} (identity: ${identity})"
		echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

		# Get all canisters from the environment file (they use "ic" as the network key)
		local canisters
		canisters=$(jq -r 'to_entries[] | select(.value.ic != null) | .key' "${env_file}" 2>/dev/null)

		if [[ -z ${canisters} ]]; then
			log_warn "  No canisters found for ${env}"
			echo ""
			continue
		fi

		# Table header
		printf "  %-25s %-30s %s\n" "CANISTER" "ID" "CYCLES"
		printf "  %-25s %-30s %s\n" "-------------------------" "------------------------------" "---------------"

		for canister in ${canisters}; do
			local canister_id
			canister_id=$(jq -r ".\"${canister}\".ic // empty" "${env_file}")

			if [[ -z ${canister_id} ]]; then
				continue
			fi

			# Get cycles balance using canister ID directly with --network ic
			local cycles_output
			local cycles_balance="error"

			if cycles_output=$(dfx canister status "${canister_id}" --network ic 2>&1); then
				# Extract balance from output like "Balance: 1_234_567_890 Cycles"
				cycles_balance=$(echo "${cycles_output}" | grep -oP 'Balance: \K[0-9_]+(?= Cycles)' | tr -d '_')
				if [[ -n ${cycles_balance} ]]; then
					# Format with T/B/M suffix using awk for floating point
					if [[ ${cycles_balance} -ge 1000000000000 ]]; then
						cycles_balance="$(awk "BEGIN {printf \"%.2f\", ${cycles_balance} / 1000000000000}") TC"
					elif [[ ${cycles_balance} -ge 1000000000 ]]; then
						cycles_balance="$(awk "BEGIN {printf \"%.2f\", ${cycles_balance} / 1000000000}") B"
					elif [[ ${cycles_balance} -ge 1000000 ]]; then
						cycles_balance="$(awk "BEGIN {printf \"%.2f\", ${cycles_balance} / 1000000}") M"
					else
						cycles_balance="${cycles_balance} cycles"
					fi
				else
					cycles_balance="unknown"
				fi
			else
				# Check if it's an out-of-cycles error
				if echo "${cycles_output}" | grep -q "out of cycles"; then
					cycles_balance="${RED}OUT OF CYCLES${NC}"
				else
					cycles_balance="${RED}error${NC}"
				fi
			fi

			printf "  %-25s %-30s %b\n" "${canister}" "${canister_id}" "${cycles_balance}"
		done
		echo ""
	done

	# Also show cycles ledger balance
	echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	echo -e "${BLUE}Cycles Ledger Balances${NC}"
	echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	printf "  %-25s %s\n" "IDENTITY" "BALANCE"
	printf "  %-25s %s\n" "-------------------------" "---------------"

	for env in "${environments[@]}"; do
		local identity="${NETWORK_IDENTITIES[$env]}"
		if dfx identity list 2>/dev/null | grep -q "^${identity}"; then
			dfx identity use "${identity}" >/dev/null 2>&1
			local ledger_balance
			ledger_balance=$(dfx cycles balance --network ic 2>/dev/null || echo "error")
			printf "  %-25s %s\n" "${identity}" "${ledger_balance}"
		fi
	done
	echo ""

	# Cleanup is handled by trap, but clear it now since we're done successfully
	trap - EXIT ERR INT TERM
	cleanup_cycles
	log_info "Restored identity: ${current_identity}"
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

		# List of canisters to delete (name and env var suffix)
		local -A canisters=(
			["ic_siwa_provider"]="IC_SIWA_PROVIDER"
			["test_canister_rs"]="TEST_CANISTER_RS"
			["test_canister_ts"]="TEST_CANISTER_TS"
		)

		for canister in "${!canisters[@]}"; do
			local env_suffix="${canisters[$canister]}"
			local canister_id=""

			# Try to get canister ID from dfx first
			canister_id=$(dfx canister id "${canister}" --network "${network}" 2>/dev/null) || canister_id=""

			# If not found, try to get from .env file (useful for Juno where .dfx state may be cleared)
			if [[ -z ${canister_id} && -f "${PROJECT_ROOT}/.env" ]]; then
				canister_id=$(grep "^CANISTER_ID_${env_suffix}=" "${PROJECT_ROOT}/.env" 2>/dev/null | cut -d'=' -f2 | tr -d "'" | tr -d '"') || canister_id=""
			fi

			if [[ -n ${canister_id} ]]; then
				log_info "Deleting canister ${canister} (${canister_id})..."
				# Stop and delete using canister ID directly (works even without canister_ids.json)
				dfx canister stop "${canister_id}" --network "${network}" 2>/dev/null || true
				dfx canister delete "${canister_id}" --network "${network}" --yes 2>/dev/null || log_warn "Could not delete ${canister}"
			else
				log_info "Canister ${canister} not found on ${network}, skipping..."
			fi
		done

		# Clear .env canister entries after pruning
		if [[ -f "${PROJECT_ROOT}/.env" ]]; then
			log_info "Clearing canister IDs from .env..."
			sed -i '/^CANISTER_ID_/d' "${PROJECT_ROOT}/.env" 2>/dev/null || true
			sed -i '/^CANISTER_CANDID_PATH/d' "${PROJECT_ROOT}/.env" 2>/dev/null || true
			sed -i '/^DFX_/d' "${PROJECT_ROOT}/.env" 2>/dev/null || true
		fi
	fi

	log_info "Cleaning Cargo artifacts..."
	cargo clean

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

	cmd_candid || {
		log_error "Candid generation failed"
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

	show_canister_urls "${network}"

	log_success "Development loop complete!"
}

# Parse command line arguments
parse_args() {
	local cmd="${1:-help}"
	shift || true

	# Version sub-options
	local VERSION_ACTION=""

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
		--log-level)
			LOG_LEVEL="${2:-info}"
			shift 2
			;;
		--log-file-level)
			LOG_FILE_LEVEL="${2:-debug}"
			shift 2
			;;
		--bump)
			VERSION_ACTION="bump"
			shift
			;;
		--check)
			VERSION_ACTION="check"
			shift
			;;
		--sync)
			VERSION_ACTION="sync"
			shift
			;;
		--major)
			VERSION_ACTION="major"
			shift
			;;
		--minor)
			VERSION_ACTION="minor"
			shift
			;;
		--patch)
			VERSION_ACTION="patch"
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

	# Initialize log file
	init_log "$@"

	# Check dependencies before running command (skip for help/check)
	if [[ ${cmd} != "help" && ${cmd} != "--help" && ${cmd} != "-h" && ${cmd} != "check" ]]; then
		check_deps "${cmd}" || exit 1
	fi

	# Execute command
	case "${cmd}" in
	build)
		cmd_build
		;;
	candid)
		cmd_candid
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
	cycles)
		cmd_cycles
		;;
	loop)
		cmd_loop "${NETWORK}"
		;;
	start)
		cmd_start "${NETWORK}"
		;;
	stop)
		cmd_stop "${NETWORK}"
		;;
	logs)
		cmd_logs "${NETWORK}"
		;;
	update)
		cmd_update
		;;
	check)
		cmd_check_deps
		;;
	version)
		case "${VERSION_ACTION}" in
		bump)
			cmd_version true
			;;
		check)
			cmd_version_check
			;;
		sync)
			cmd_version_sync
			;;
		major | minor | patch)
			cmd_version_force "${VERSION_ACTION}"
			;;
		*)
			cmd_version false
			;;
		esac
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
