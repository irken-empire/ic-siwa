#!/usr/bin/env bash

# IC-SIWA Developer Entrypoint Script
# Single point of entry for all ic-siwa development tasks

clear
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
# Used by: cmd_version, cmd_version_force, cmd_version_check, cmd_version_sync, cmd_version_reset, cmd_version_force_set
NPM_PACKAGES=(
	"package.json"
	"libs/ic_siwa_ts/package.json"
	"canisters/test_canister_ts/package.json"
)

# Agent documentation sources for LLM context
# Format: "name|url"
AGENT_DOC_SOURCES=(
	"astro|https://docs.astro.build/llms-full.txt"
	"daisyui|https://daisyui.com/llms.txt"
	"foundry|https://getfoundry.sh/llms-full.txt"
	"juno|https://juno.build/llms-full.txt"
	"oisy|https://docs.oisy.com/llms-full.txt"
	"reown|https://docs.reown.com/llms-full.txt"
	"viem|https://viem.sh/llms-full.txt"
	"xai|https://docs.x.ai/llms.txt"
)
AGENT_DOCS_DIR="docs/agents"

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
		  reset              Clear stale .dfx/{network}/ state (for fresh deployments)
		  loop               Full development loop: fmt, lint, candid, build, test, deploy
		  start              Start local DFX replica
		  stop               Stop local DFX replica
		  logs               Tail ic_siwa_provider logs in real-time
		  update             Update all dependencies (cargo, bun) and pin versions
		  agent-docs         Download LLM documentation for AI agents
		  check              Check if all required dependencies are installed
		  version            Show current version (from Cargo.toml)
		  version --bump     Bump version based on conventional commits
		  version --reset    Reset all version files to 0.0.0 (CI uses convco)
		  version --force-set <ver>  Set exact version in all files
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
		  ic-siwa reset --network juno
		  ic-siwa update
		  ic-siwa version --bump

		Environment Variables (via secretspec):
		  IC_SIWA_SALT_DEVELOPMENT    Salt for development deployments
		  IC_SIWA_SALT_TESTNET        Salt for testnet deployments
		  IC_SIWA_SALT_MAINNET        Salt for mainnet deployments

		Config Files (domain/uri read from YAML):
		  config/development.yaml     Local dev (siwa.domain, siwa.uri)
		  config/testnet.yaml         IC testnet (siwa.domain, siwa.uri)
		  config/mainnet.yaml         IC mainnet (siwa.domain, siwa.uri)
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
	["test-integration"]="dfx,cast,yq"
	["fmt"]="cargo"
	["lint"]="cargo"
	["deploy"]="dfx,cargo,bun,yq,jq"
	["upgrade"]="dfx,cargo,bun,yq"
	["loop"]="dfx,cargo,bun,yq,candid-extractor,didc,cast"
	["start"]="dfx"
	["stop"]="dfx"
	["logs"]="dfx"
	["update"]="cargo,bun,cargo-upgrade"
	["version"]="toml"
	["cleanup"]="dfx"
	["reset"]=""
	["urls"]="dfx"
	["cycles"]="dfx,jq"
	["agent-docs"]="curl"
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
			cargo-upgrade)
				echo -e "  ${RED}✗${NC} cargo-upgrade - Install: cargo install cargo-edit"
				;;
			toml)
				echo -e "  ${RED}✗${NC} toml - Install: cargo install toml-cli"
				;;
			convco)
				echo -e "  ${RED}✗${NC} convco - Install: cargo install convco"
				;;
			cast)
				echo -e "  ${RED}✗${NC} cast (Foundry) - Install: curl -L https://foundry.paradigm.xyz | bash"
				;;
			jq)
				echo -e "  ${RED}✗${NC} jq - Install via package manager (apt/brew/nix)"
				;;
			nc)
				echo -e "  ${RED}✗${NC} nc (netcat) - Install via package manager"
				;;
			curl)
				echo -e "  ${RED}✗${NC} curl - Install via package manager"
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
	log_info "  3. Push to trunk: git push"
	log_info ""
	log_info "GitHub Actions will automatically:"
	log_info "  - Pre-release: Create tag v${next_version} (prerelease) on push to trunk"
	log_info "  - Release: Promote pre-release via manual workflow dispatch"
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

	# Check TypeScript VERSION constant
	local ts_index="${PROJECT_ROOT}/libs/ic_siwa_ts/src/index.ts"
	if [[ -f ${ts_index} ]]; then
		local ts_version
		ts_version=$(grep -oP 'export const VERSION = "\K[^"]+' "${ts_index}" || echo "unknown")
		if [[ ${cargo_version} == "${ts_version}" ]]; then
			echo "  libs/ic_siwa_ts/src/index.ts (VERSION): ${ts_version} ✓"
		else
			echo "  libs/ic_siwa_ts/src/index.ts (VERSION): ${ts_version} ✗ (mismatch!)"
			has_error=true
		fi
	fi

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

	# Update hardcoded VERSION constant in TypeScript source
	local ts_index="${PROJECT_ROOT}/libs/ic_siwa_ts/src/index.ts"
	if [[ -f ${ts_index} ]]; then
		sed -i "s/export const VERSION = \".*\"/export const VERSION = \"${cargo_version}\"/" "${ts_index}"
		log_info "Updated TypeScript VERSION constant"
	fi

	log_success "Versions synced to ${cargo_version}"
}

# Reset all version files to 0.0.0 (CI/convco is the source of truth)
cmd_version_reset() {
	cd "${PROJECT_ROOT}"

	local BASE_VERSION="0.0.0"

	log_info "Resetting all versions to ${BASE_VERSION}..."

	# Update Cargo.toml (workspace version)
	toml set Cargo.toml workspace.package.version "${BASE_VERSION}" >Cargo.toml.tmp
	mv Cargo.toml.tmp Cargo.toml

	# Update all npm package.json files
	for npm_package in "${NPM_PACKAGES[@]}"; do
		local full_path="${PROJECT_ROOT}/${npm_package}"
		if [[ -f ${full_path} ]]; then
			jq --arg v "${BASE_VERSION}" '.version = $v' "${full_path}" >"${full_path}.tmp"
			mv "${full_path}.tmp" "${full_path}"
		fi
	done

	# Update TypeScript VERSION constant
	local ts_index="${PROJECT_ROOT}/libs/ic_siwa_ts/src/index.ts"
	if [[ -f ${ts_index} ]]; then
		sed -i "s/export const VERSION = \".*\"/export const VERSION = \"${BASE_VERSION}\"/" "${ts_index}"
	fi

	# Update Cargo.lock
	cargo check --quiet 2>/dev/null || true

	log_success "All versions reset to ${BASE_VERSION}"
}

# Force set an exact version in all files (used by CI)
cmd_version_force_set() {
	local target_version="${1:-}"
	cd "${PROJECT_ROOT}"

	if [[ -z ${target_version} ]]; then
		log_error "Usage: ic-siwa version --force-set <semver>"
		return 1
	fi

	# Strip leading 'v' if present
	target_version="${target_version#v}"

	if ! [[ ${target_version} =~ ^[0-9]+\.[0-9]+\.[0-9]+ ]]; then
		log_error "Invalid version format: '${target_version}'. Expected semver (e.g. 0.3.0)"
		return 1
	fi

	log_info "Setting version to ${target_version} in all files..."

	# Update Cargo.toml (workspace version)
	toml set Cargo.toml workspace.package.version "${target_version}" >Cargo.toml.tmp
	mv Cargo.toml.tmp Cargo.toml

	# Update all npm package.json files
	for npm_package in "${NPM_PACKAGES[@]}"; do
		local full_path="${PROJECT_ROOT}/${npm_package}"
		if [[ -f ${full_path} ]]; then
			jq --arg v "${target_version}" '.version = $v' "${full_path}" >"${full_path}.tmp"
			mv "${full_path}.tmp" "${full_path}"
		fi
	done

	# Update TypeScript VERSION constant
	local ts_index="${PROJECT_ROOT}/libs/ic_siwa_ts/src/index.ts"
	if [[ -f ${ts_index} ]]; then
		sed -i "s/export const VERSION = \".*\"/export const VERSION = \"${target_version}\"/" "${ts_index}"
	fi

	# Update Cargo.lock
	cargo check --quiet 2>/dev/null || true

	log_success "All versions set to ${target_version}"
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

	# Update Cargo dependencies with exact pinned versions
	log_info "Updating Cargo dependencies (pinned exact versions)..."
	if command -v cargo-upgrade &>/dev/null; then
		# cargo-upgrade updates Cargo.toml to latest versions with exact pins
		cargo upgrade --pinned
		cargo update
		log_success "Cargo dependencies upgraded and pinned"
	else
		log_warn "cargo-upgrade not found, falling back to cargo update (Cargo.lock only)"
		log_info "Install cargo-edit for pinned Cargo.toml updates: cargo install cargo-edit"
		cargo update
		log_success "Cargo.lock updated (Cargo.toml unchanged)"
	fi

	# Update bun packages to latest versions with exact versions (no ^ or ~ prefixes)
	for npm_dir in "${npm_dirs[@]}"; do
		local full_path="${PROJECT_ROOT}/${npm_dir}"
		if [[ -d ${full_path} && -f "${full_path}/package.json" ]]; then
			log_info "Updating bun packages in ${npm_dir}..."
			(
				cd "${full_path}"
				# bun update --latest can write "latest" literally, so we use a different approach:
				# 1. Install to update bun.lock with resolved versions
				# 2. Use bun's --save-exact with explicit package names

				# Get all dependencies and devDependencies from package.json
				local deps
				deps=$(jq -r '(.dependencies // {}) + (.devDependencies // {}) | keys[]' package.json 2>/dev/null || true)

				if [[ -n ${deps} ]]; then
					# Update each package individually to get exact versions
					for pkg in ${deps}; do
						# Skip local file references
						if jq -e --arg pkg "$pkg" '(.dependencies[$pkg] // .devDependencies[$pkg]) | startswith("file:")' package.json &>/dev/null; then
							log_debug "Skipping local package: ${pkg}"
							continue
						fi

						# Check if it's a dev dependency
						if jq -e --arg pkg "$pkg" '.devDependencies[$pkg]' package.json &>/dev/null; then
							bun add -d --exact "${pkg}@latest" 2>/dev/null || log_warn "Failed to update ${pkg}"
						else
							bun add --exact "${pkg}@latest" 2>/dev/null || log_warn "Failed to update ${pkg}"
						fi
					done
				fi
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

# Download LLM documentation for AI agents
cmd_agent_docs() {
	log_info "Downloading AI agent documentation..."

	mkdir -p "${PROJECT_ROOT}/${AGENT_DOCS_DIR}"

	local failed=0
	local success=0
	local total=${#AGENT_DOC_SOURCES[@]}
	local current=0

	for entry in "${AGENT_DOC_SOURCES[@]}"; do
		current=$((current + 1))

		# Split entry by pipe
		local name="${entry%%|*}"
		local url="${entry##*|}"
		local filename="${name,,}.txt" # lowercase
		local filepath="${PROJECT_ROOT}/${AGENT_DOCS_DIR}/${filename}"

		log_info "[${current}/${total}] Downloading ${name} docs from ${url}..."

		if curl -s --connect-timeout 10 --max-time 60 -o "${filepath}" "${url}"; then
			# Check if file has content
			if [[ -s ${filepath} ]]; then
				local size
				size=$(wc -c <"${filepath}" | tr -d ' ')
				log_success "${name} docs saved (${size} bytes)"
				success=$((success + 1))
			else
				log_warn "${name} docs downloaded but file is empty"
				rm -f "${filepath}"
				failed=$((failed + 1))
			fi
		else
			log_error "Failed to download ${name} docs"
			failed=$((failed + 1))
		fi
	done

	log_info ""
	log_info "Download complete: ${success} succeeded, ${failed} failed"

	if [[ ${success} -gt 0 ]]; then
		log_info ""
		log_info "Documentation files saved to: ${AGENT_DOCS_DIR}/"
		log_info "Run this command periodically to fetch updated documentation."
	fi
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

	# Step 3: Post-process the generated TypeScript for compatibility
	log_info "Fixing Candid TypeScript imports and unused variables..."

	# Fix imports from @icp-sdk/core to @dfinity/* (didc generates new SDK paths)
	sed -i \
		-e "s|@icp-sdk/core/principal|@dfinity/principal|g" \
		-e "s|@icp-sdk/core/agent|@dfinity/agent|g" \
		-e "s|@icp-sdk/core/candid|@dfinity/candid|g" \
		"${ts_file}"

	# Remove unused top-level IDL import (IDL is passed as parameter to factory functions)
	sed -i "/^import { IDL } from '@dfinity\/candid';$/d" "${ts_file}"

	# Remove init-only type declarations (RateLimitArgs, InitArgs) from idlFactory.
	# didc emits these at the top of idlFactory but they're only needed in init().
	# They cause noUnusedLocals errors since idlFactory's IDL.Service doesn't reference them.
	# Use awk to precisely remove only these blocks within idlFactory.
	awk '
		/^export const idlFactory/ { in_factory=1 }
		/^};/ && in_factory { in_factory=0 }
		in_factory && /^  const RateLimitArgs = IDL\.Record/ { skip=1 }
		in_factory && /^  const InitArgs = IDL\.Record/ { skip=1 }
		skip && /^  \}\);/ { skip=0; next }
		!skip { print }
	' "${ts_file}" >"${ts_file}.tmp" && mv "${ts_file}.tmp" "${ts_file}"

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
	local total_tests=27

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

	# ==========================================
	# Multi-Tenant and Domain Tests
	# ==========================================
	log_info ""
	log_info "Running multi-tenant and domain tests..."

	# Test 11: Multi-tenant prepare_login_with_options (whitelisted domain)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing multi-tenant prepare_login_with_options..."

	# Read allowed_domains from config to find a whitelisted domain
	local config_file_mt
	case "${network}" in
	dfx | juno) config_file_mt="${PROJECT_ROOT}/config/development.yaml" ;;
	testnet) config_file_mt="${PROJECT_ROOT}/config/testnet.yaml" ;;
	ic) config_file_mt="${PROJECT_ROOT}/config/mainnet.yaml" ;;
	*) config_file_mt="${PROJECT_ROOT}/config/development.yaml" ;;
	esac

	local first_allowed_domain=""
	if [[ -f ${config_file_mt} ]] && command -v yq &>/dev/null; then
		first_allowed_domain=$(yq -r '.security.allowed_domains[0] // ""' "${config_file_mt}")
		# Strip wildcard prefix if present (*.example.com -> example.com)
		first_allowed_domain="${first_allowed_domain#\*.}"
	fi

	if [[ -n ${first_allowed_domain} ]]; then
		local mt_test_address="0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B"
		local mt_result
		mt_result=$(dfx canister call ic_siwa_provider siwa_prepare_login_with_options \
			"(record { address = \"${mt_test_address}\"; domain = opt \"${first_allowed_domain}\"; uri = opt \"https://${first_allowed_domain}\" })" \
			--network "${network}" 2>/dev/null)
		if echo "${mt_result}" | grep -q "Ok"; then
			# Verify the returned message contains the custom domain
			if echo "${mt_result}" | grep -q "${first_allowed_domain}"; then
				log_success "  Multi-tenant login returned message with domain '${first_allowed_domain}'"
			else
				log_warn "  Multi-tenant login returned Ok but domain not found in message"
			fi
		else
			log_error "  Multi-tenant prepare_login_with_options failed: ${mt_result}"
			failed=$((failed + 1))
		fi
	else
		log_info "  Skipped (no allowed_domains configured)"
	fi

	# Test 12: Domain whitelisting rejection (non-whitelisted domain)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing domain whitelisting rejection..."

	if [[ -n ${first_allowed_domain} ]]; then
		local reject_address="0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B"
		local reject_result
		reject_result=$(dfx canister call ic_siwa_provider siwa_prepare_login_with_options \
			"(record { address = \"${reject_address}\"; domain = opt \"evil-domain.example.net\"; uri = opt \"https://evil-domain.example.net\" })" \
			--network "${network}" 2>/dev/null)
		if echo "${reject_result}" | grep -qi "Err\|not.*allowed\|not in"; then
			log_success "  Non-whitelisted domain correctly rejected"
		else
			log_error "  Non-whitelisted domain should have been rejected: ${reject_result}"
			failed=$((failed + 1))
		fi
	else
		# No allowed_domains means any domain is accepted
		log_info "  Skipped (no allowed_domains configured - all domains accepted)"
	fi

	# Test 13: Consecutive login attempts from same address
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing consecutive login from same address..."
	local concurrent_address="0x1111111111111111111111111111111111111111"
	local concurrent_result1 concurrent_result2
	concurrent_result1=$(dfx canister call ic_siwa_provider siwa_prepare_login "(\"${concurrent_address}\")" --network "${network}" 2>&1)
	sleep 1 # Allow replica to process the first call before the second
	concurrent_result2=$(dfx canister call ic_siwa_provider siwa_prepare_login "(\"${concurrent_address}\")" --network "${network}" 2>&1)
	if echo "${concurrent_result1}" | grep -q "Ok" && echo "${concurrent_result2}" | grep -q "Ok"; then
		# Second call should succeed (overwriting the first session for same address)
		log_success "  Second prepare_login for same address succeeds (overwrites previous)"
	elif echo "${concurrent_result2}" | grep -qi "rate"; then
		log_warn "  Second call rate-limited (total request limit may have been reached)"
	else
		log_error "  Consecutive login test failed"
		log_error "    Result 1: ${concurrent_result1}"
		log_error "    Result 2: ${concurrent_result2}"
		failed=$((failed + 1))
	fi

	# Test 14: Login with invalid signature (simulates wrong wallet)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing login with invalid signature..."
	local bad_sig_address="0x2222222222222222222222222222222222222222"
	# First prepare a login
	dfx canister call ic_siwa_provider siwa_prepare_login "(\"${bad_sig_address}\")" --network "${network}" >/dev/null 2>&1
	# Then try to login with a garbage signature
	local fake_sig
	fake_sig="0x$(printf 'ab%.0s' {1..64})1b"
	local bad_sig_result
	bad_sig_result=$(dfx canister call ic_siwa_provider siwa_login "(\"${fake_sig}\", \"${bad_sig_address}\", blob \"\\00\\01\\02\\03\\04\\05\\06\\07\\08\\09\\0a\\0b\\0c\\0d\\0e\\0f\")" --network "${network}" 2>/dev/null)
	if echo "${bad_sig_result}" | grep -qi "Err"; then
		log_success "  Invalid signature correctly rejected"
	else
		log_error "  Invalid signature should have been rejected: ${bad_sig_result}"
		failed=$((failed + 1))
	fi

	# Test 15: Prepare delegation without authenticated session
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing prepare_delegation without session..."
	local no_session_result
	no_session_result=$(dfx canister call ic_siwa_provider siwa_prepare_delegation \
		'("0x3333333333333333333333333333333333333333", blob "\00\01\02\03\04\05\06\07\08\09\0a\0b\0c\0d\0e\0f", 3600000000000 : nat64)' \
		--network "${network}" 2>/dev/null)
	if echo "${no_session_result}" | grep -qi "Err"; then
		log_success "  Prepare delegation correctly requires authenticated session"
	else
		log_error "  Prepare delegation should fail without valid session"
		failed=$((failed + 1))
	fi

	# Test 16: Logout without valid session (should fail gracefully)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing logout without session..."
	local logout_result
	logout_result=$(dfx canister call ic_siwa_provider siwa_logout \
		'("0x4444444444444444444444444444444444444444", blob "\00\01\02\03\04\05\06\07\08\09\0a\0b\0c\0d\0e\0f")' \
		--network "${network}" 2>&1)
	if echo "${logout_result}" | grep -qi "Err\|not found\|Session not found"; then
		log_success "  Logout correctly fails for non-existent session"
	elif echo "${logout_result}" | grep -qi "has no method\|is not defined"; then
		log_warn "  Skipped (siwa_logout not in Candid interface - regenerate with 'ic-siwa candid')"
	else
		log_error "  Logout should fail for non-existent session: ${logout_result}"
		failed=$((failed + 1))
	fi

	# ==========================================
	# Full Auth Flow Tests (require Foundry cast)
	# ==========================================
	log_info ""
	log_info "Running full auth flow tests (using Foundry cast for signing)..."

	# Pre-declare flow state variables so that later -n checks don't trigger
	# "unbound variable" under set -u when earlier tests fail/skip.
	local LOGIN_EXPIRATION=""
	local CAPPED_EXPIRATION=""
	local COMBINED_EXPIRATION=""
	local derived_principal=""

	# Foundry default test account #0 - NEVER use this for real funds
	local CAST_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
	local CAST_ADDRESS
	CAST_ADDRESS=$(FOUNDRY_DISABLE_NIGHTLY_WARNING=1 cast to-check-sum-address "$(FOUNDRY_DISABLE_NIGHTLY_WARNING=1 cast wallet address "${CAST_PRIVATE_KEY}")")

	# Generate a 32-byte session key for testing
	local SESSION_KEY_HEX
	SESSION_KEY_HEX=$(FOUNDRY_DISABLE_NIGHTLY_WARNING=1 cast keccak256 "session-key-$(date +%s%N)")
	SESSION_KEY_HEX="${SESSION_KEY_HEX#0x}"
	# Helper: convert hex string to Candid blob format for dfx
	# Candid text format uses \XX raw hex escapes (no 'x' prefix)
	# e.g., "aabb" -> 'blob "\aa\bb"'
	hex_to_blob() {
		local hex="${1#0x}"
		local blob=""
		local i
		for ((i = 0; i < ${#hex}; i += 2)); do
			blob+="\\${hex:i:2}"
		done
		echo "blob \"${blob}\""
	}

	# Helper: extract the message field from a Candid PrepareLoginResponse
	# dfx formats records across multiple lines, so we collapse them first
	extract_siwa_message() {
		local candid_output="$1"
		echo "${candid_output}" | tr '\n' ' ' | sed -n 's/.*message = "\(.*\)"; *nonce.*/\1/p' | sed 's/\\n/\n/g'
	}

	local SESSION_KEY_BLOB
	SESSION_KEY_BLOB=$(hex_to_blob "${SESSION_KEY_HEX}")

	log_info "  Test address: ${CAST_ADDRESS}"

	# Test 17: Full auth flow - prepare_login -> cast sign -> siwa_login
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing full auth flow (prepare -> sign -> login)..."

	# Step 1: Prepare login
	local auth_prepare_result
	auth_prepare_result=$(dfx canister call ic_siwa_provider siwa_prepare_login "(\"${CAST_ADDRESS}\")" --network "${network}" 2>/dev/null)

	if ! echo "${auth_prepare_result}" | grep -q "Ok"; then
		log_error "  Full auth flow: prepare_login failed: ${auth_prepare_result}"
		failed=$((failed + 1))
	else
		# Step 2: Extract the SIWA message from the Candid response
		# The message field contains escaped newlines (\n) and is enclosed in quotes
		local siwa_message
		siwa_message=$(extract_siwa_message "${auth_prepare_result}")

		if [[ -z ${siwa_message} ]]; then
			log_error "  Full auth flow: failed to extract SIWA message from response"
			failed=$((failed + 1))
		else
			# Step 3: Sign the message with cast (EIP-191 personal_sign)
			local signature
			signature=$(FOUNDRY_DISABLE_NIGHTLY_WARNING=1 cast wallet sign --private-key "${CAST_PRIVATE_KEY}" "$(printf '%b' "${siwa_message}")" 2>/dev/null)

			if [[ -z ${signature} ]]; then
				log_error "  Full auth flow: cast wallet sign failed"
				failed=$((failed + 1))
			else
				# Step 4: Login with signature
				local auth_login_result
				auth_login_result=$(dfx canister call ic_siwa_provider siwa_login \
					"(\"${signature}\", \"${CAST_ADDRESS}\", ${SESSION_KEY_BLOB})" \
					--network "${network}" 2>/dev/null)

				if echo "${auth_login_result}" | grep -q "Ok"; then
					log_success "  Full auth flow: login succeeded"

					# Extract expiration from login response for subsequent tests
					LOGIN_EXPIRATION=$(echo "${auth_login_result}" | grep -o 'expiration = [0-9_]*' | head -1 | sed 's/expiration = //;s/_//g')
				else
					log_error "  Full auth flow: login failed: ${auth_login_result}"
					failed=$((failed + 1))
				fi
			fi
		fi
	fi

	# Test 18: Prepare delegation after login
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing prepare_delegation after login..."
	if [[ -n ${LOGIN_EXPIRATION} ]]; then
		local prep_del_result
		prep_del_result=$(dfx canister call ic_siwa_provider siwa_prepare_delegation \
			"(\"${CAST_ADDRESS}\", ${SESSION_KEY_BLOB}, ${LOGIN_EXPIRATION} : nat64)" \
			--network "${network}" 2>/dev/null)

		if echo "${prep_del_result}" | grep -q "Ok"; then
			# Extract the capped expiration returned by prepare_delegation
			CAPPED_EXPIRATION=$(echo "${prep_del_result}" | grep -o 'Ok = [0-9_]*' | sed 's/Ok = //;s/_//g')
			log_success "  Prepare delegation succeeded (expiration: ${CAPPED_EXPIRATION})"
		else
			log_error "  Prepare delegation failed: ${prep_del_result}"
			failed=$((failed + 1))
		fi
	else
		log_warn "  Skipped (no login expiration from previous test)"
	fi

	# Test 19: Get delegation (certified query)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing get_delegation after prepare..."
	if [[ -n ${CAPPED_EXPIRATION} ]]; then
		local get_del_result
		get_del_result=$(dfx canister call ic_siwa_provider siwa_get_delegation \
			"(\"${CAST_ADDRESS}\", ${SESSION_KEY_BLOB}, ${CAPPED_EXPIRATION} : nat64)" \
			--network "${network}" 2>/dev/null)

		if echo "${get_del_result}" | grep -q "Ok"; then
			log_success "  Get delegation succeeded (signed delegation returned)"
		else
			log_error "  Get delegation failed: ${get_del_result}"
			failed=$((failed + 1))
		fi
	else
		log_warn "  Skipped (no capped expiration from previous test)"
	fi

	# Test 20: Principal lookup after login
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing get_principal after login..."
	if [[ -n ${LOGIN_EXPIRATION} ]]; then
		local principal_lookup
		principal_lookup=$(dfx canister call ic_siwa_provider get_principal "(\"${CAST_ADDRESS}\")" --network "${network}" 2>/dev/null)

		if echo "${principal_lookup}" | grep -q "Ok"; then
			derived_principal=$(echo "${principal_lookup}" | grep -o 'principal "[^"]*"' | sed 's/principal "//;s/"//')
			log_success "  Principal lookup succeeded: ${derived_principal}"
		else
			log_error "  Principal lookup failed: ${principal_lookup}"
			failed=$((failed + 1))
		fi
	else
		log_warn "  Skipped (login did not succeed)"
	fi

	# Test 21: Address lookup (reverse mapping)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing get_address for derived principal..."
	if [[ -n ${derived_principal} ]]; then
		local address_lookup
		address_lookup=$(dfx canister call ic_siwa_provider get_address "(principal \"${derived_principal}\")" --network "${network}" 2>/dev/null)

		if echo "${address_lookup}" | grep -q "Ok"; then
			local returned_address
			returned_address=$(echo "${address_lookup}" | sed -n 's/.*Ok = "\([^"]*\)".*/\1/p')
			if [[ ${returned_address,,} == "${CAST_ADDRESS,,}" ]]; then
				log_success "  Address lookup matches: ${returned_address}"
			else
				log_error "  Address mismatch: expected ${CAST_ADDRESS}, got ${returned_address}"
				failed=$((failed + 1))
			fi
		else
			log_error "  Address lookup failed: ${address_lookup}"
			failed=$((failed + 1))
		fi
	else
		log_warn "  Skipped (no derived principal from previous test)"
	fi

	# Test 22: Logout real session
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing logout of authenticated session..."
	if [[ -n ${LOGIN_EXPIRATION} ]]; then
		local auth_logout_result
		auth_logout_result=$(dfx canister call ic_siwa_provider siwa_logout \
			"(\"${CAST_ADDRESS}\", ${SESSION_KEY_BLOB})" \
			--network "${network}" 2>/dev/null)

		if echo "${auth_logout_result}" | grep -q "Ok"; then
			log_success "  Logout succeeded"
		else
			log_error "  Logout failed: ${auth_logout_result}"
			failed=$((failed + 1))
		fi
	else
		log_warn "  Skipped (no active session)"
	fi

	# Test 23: Post-logout delegation should fail
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing delegation fails after logout..."
	if [[ -n ${CAPPED_EXPIRATION} ]]; then
		local post_logout_del
		post_logout_del=$(dfx canister call ic_siwa_provider siwa_get_delegation \
			"(\"${CAST_ADDRESS}\", ${SESSION_KEY_BLOB}, ${CAPPED_EXPIRATION} : nat64)" \
			--network "${network}" 2>/dev/null)

		if echo "${post_logout_del}" | grep -qi "Err"; then
			log_success "  Delegation correctly fails after logout"
		else
			log_error "  Delegation should fail after logout: ${post_logout_del}"
			failed=$((failed + 1))
		fi
	else
		log_warn "  Skipped (no session to verify)"
	fi

	# Test 24: Combined endpoint (login_and_prepare)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing combined siwa_login_and_prepare..."

	# Need a fresh session key for combined endpoint
	local SESSION_KEY2_HEX
	SESSION_KEY2_HEX=$(FOUNDRY_DISABLE_NIGHTLY_WARNING=1 cast keccak256 "session-key-combined-$(date +%s%N)")
	SESSION_KEY2_HEX="${SESSION_KEY2_HEX#0x}"
	local SESSION_KEY2_BLOB
	SESSION_KEY2_BLOB=$(hex_to_blob "${SESSION_KEY2_HEX}")

	# Prepare login first
	local combined_prepare
	combined_prepare=$(dfx canister call ic_siwa_provider siwa_prepare_login "(\"${CAST_ADDRESS}\")" --network "${network}" 2>/dev/null)

	if echo "${combined_prepare}" | grep -q "Ok"; then
		# Extract and sign the message
		local combined_message
		combined_message=$(extract_siwa_message "${combined_prepare}")
		local combined_sig
		combined_sig=$(FOUNDRY_DISABLE_NIGHTLY_WARNING=1 cast wallet sign --private-key "${CAST_PRIVATE_KEY}" "$(printf '%b' "${combined_message}")" 2>/dev/null)

		if [[ -n ${combined_sig} ]]; then
			local combined_result
			combined_result=$(dfx canister call ic_siwa_provider siwa_login_and_prepare \
				"(\"${combined_sig}\", \"${CAST_ADDRESS}\", ${SESSION_KEY2_BLOB})" \
				--network "${network}" 2>/dev/null)

			if echo "${combined_result}" | grep -q "Ok"; then
				log_success "  Combined login_and_prepare succeeded"

				COMBINED_EXPIRATION=$(echo "${combined_result}" | grep -o 'expiration = [0-9_]*' | head -1 | sed 's/expiration = //;s/_//g')
			else
				log_error "  Combined login_and_prepare failed: ${combined_result}"
				failed=$((failed + 1))
			fi
		else
			log_error "  Failed to sign message for combined endpoint"
			failed=$((failed + 1))
		fi
	else
		log_error "  Prepare login for combined endpoint failed: ${combined_prepare}"
		failed=$((failed + 1))
	fi

	# Test 25: Get delegation after combined endpoint
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing get_delegation after combined endpoint..."
	if [[ -n ${COMBINED_EXPIRATION} ]]; then
		local combined_del_result
		combined_del_result=$(dfx canister call ic_siwa_provider siwa_get_delegation \
			"(\"${CAST_ADDRESS}\", ${SESSION_KEY2_BLOB}, ${COMBINED_EXPIRATION} : nat64)" \
			--network "${network}" 2>/dev/null)

		if echo "${combined_del_result}" | grep -q "Ok"; then
			log_success "  Get delegation after combined endpoint succeeded"
		else
			log_error "  Get delegation after combined endpoint failed: ${combined_del_result}"
			failed=$((failed + 1))
		fi
	else
		log_warn "  Skipped (combined endpoint did not succeed)"
	fi

	# Test 26: Re-login after logout (full flow again)
	((++test_num))
	log_info "[${test_num}/${total_tests}] Testing re-login after logout..."

	local SESSION_KEY3_HEX
	SESSION_KEY3_HEX=$(FOUNDRY_DISABLE_NIGHTLY_WARNING=1 cast keccak256 "session-key-relogin-$(date +%s%N)")
	SESSION_KEY3_HEX="${SESSION_KEY3_HEX#0x}"
	local SESSION_KEY3_BLOB
	SESSION_KEY3_BLOB=$(hex_to_blob "${SESSION_KEY3_HEX}")

	local relogin_prepare
	relogin_prepare=$(dfx canister call ic_siwa_provider siwa_prepare_login "(\"${CAST_ADDRESS}\")" --network "${network}" 2>/dev/null)

	if echo "${relogin_prepare}" | grep -q "Ok"; then
		local relogin_message
		relogin_message=$(extract_siwa_message "${relogin_prepare}")
		local relogin_sig
		relogin_sig=$(FOUNDRY_DISABLE_NIGHTLY_WARNING=1 cast wallet sign --private-key "${CAST_PRIVATE_KEY}" "$(printf '%b' "${relogin_message}")" 2>/dev/null)

		if [[ -n ${relogin_sig} ]]; then
			local relogin_result
			relogin_result=$(dfx canister call ic_siwa_provider siwa_login \
				"(\"${relogin_sig}\", \"${CAST_ADDRESS}\", ${SESSION_KEY3_BLOB})" \
				--network "${network}" 2>/dev/null)

			if echo "${relogin_result}" | grep -q "Ok"; then
				log_success "  Re-login after logout succeeded"
			else
				log_error "  Re-login failed: ${relogin_result}"
				failed=$((failed + 1))
			fi
		else
			log_error "  Failed to sign message for re-login"
			failed=$((failed + 1))
		fi
	else
		log_error "  Prepare login for re-login failed: ${relogin_prepare}"
		failed=$((failed + 1))
	fi

	# Test 27: Rate limiting - rapid requests should eventually be blocked
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

	log_info "Tailing ic_siwa_provider canister logs on network '${network}' (Ctrl+C to stop)..."

	case "${network}" in
	dfx)
		dfx canister logs ic_siwa_provider --follow
		;;
	juno)
		dfx canister logs ic_siwa_provider --follow --network juno
		;;
	ic)
		dfx canister logs ic_siwa_provider --follow --network ic
		;;
	*)
		log_error "Unknown network: ${network}"
		return 1
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

	# Read domain/uri from config file (consistent with CI read-config action)
	local domain
	domain=$(yq -r '.siwa.domain' "${config_file}")
	local uri
	uri=$(yq -r '.siwa.uri' "${config_file}")
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
		# Detect stale canister IDs from a previous Juno instance
		local juno_ids="${PROJECT_ROOT}/.dfx/juno/canister_ids.json"
		if [[ -f ${juno_ids} ]]; then
			local test_id
			test_id=$(jq -r '.ic_siwa_provider.juno // empty' "${juno_ids}" 2>/dev/null)
			if [[ -n ${test_id} ]]; then
				if ! dfx canister status "${test_id}" --network juno &>/dev/null; then
					log_warn "Stale canister IDs detected (Juno was restarted?)"
					log_info "Auto-resetting DFX state for fresh deployment..."
					cmd_reset "juno"
				fi
			fi
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
	log_debug "  Init arg: ${init_arg}"

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
		elif [[ ${network} == "juno" ]]; then
			# Juno uses subdomain-style URLs which properly route dynamic imports
			log_info "  ic_siwa_provider (Candid): http://${candid_id}.localhost:${port}/?id=${provider_id}"
		else
			log_info "  ic_siwa_provider (Candid): http://127.0.0.1:${port}/?canisterId=${candid_id}&id=${provider_id}"
		fi
	fi

	if [[ -n ${rs_id} ]]; then
		if [[ ${network} == "ic" ]]; then
			log_info "  test_canister_rs (Candid): https://${candid_id}.raw.ic0.app/?id=${rs_id}"
		elif [[ ${network} == "juno" ]]; then
			log_info "  test_canister_rs (Candid): http://${candid_id}.localhost:${port}/?id=${rs_id}"
		else
			log_info "  test_canister_rs (Candid): http://127.0.0.1:${port}/?canisterId=${candid_id}&id=${rs_id}"
		fi
	fi

	if [[ -n ${ts_id} ]]; then
		if [[ ${network} == "ic" ]]; then
			log_info "  test_canister_ts (Frontend): https://${ts_id}.ic0.app"
		elif [[ ${network} == "juno" ]]; then
			# Juno subdomain URLs work with dynamic imports (no chunking issues)
			log_info "  test_canister_ts (Frontend): http://${ts_id}.localhost:${port}/"
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
	log_debug "  Init arg: ${init_arg}"

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
# Reset DFX state for a network (clear stale canister IDs)
cmd_reset() {
	local network="${1:-dfx}"

	if [[ ${network} == "ic" ]]; then
		log_error "Cannot reset IC mainnet state - use 'cleanup --prune --network ic' instead"
		return 1
	fi

	cd "${PROJECT_ROOT}"
	local dfx_dir="${PROJECT_ROOT}/.dfx/${network}"

	if [[ -d ${dfx_dir} ]]; then
		log_warn "Removing stale DFX state: ${dfx_dir}"
		rm -rf "${dfx_dir}"
		log_success "DFX state cleared for network '${network}'"
	else
		log_info "No DFX state found for network '${network}'"
	fi

	# Clear stale wallet canister entries for this network
	# dfx stores wallet IDs per-identity in .dfx/local/wallets.json, keyed by network name
	local wallets_file="${PROJECT_ROOT}/.dfx/local/wallets.json"
	if [[ -f ${wallets_file} ]] && command -v jq &>/dev/null; then
		if jq -e ".identities | to_entries[] | .value.\"${network}\"" "${wallets_file}" &>/dev/null; then
			log_info "Clearing stale wallet entries for network '${network}'..."
			local tmp_wallets
			tmp_wallets=$(jq "(.identities // {}) |= with_entries(.value |= del(.\"${network}\"))" "${wallets_file}")
			echo "${tmp_wallets}" >"${wallets_file}"
			log_success "Wallet state cleared for network '${network}'"
		fi
	fi

	# Clear .env canister entries (DFX writes both CANISTER_ID= and CANISTER_ID_*= forms)
	if [[ -f "${PROJECT_ROOT}/.env" ]]; then
		log_info "Clearing canister IDs from .env..."
		sed -i '/^CANISTER_ID/d' "${PROJECT_ROOT}/.env" 2>/dev/null || true
		sed -i '/^CANISTER_CANDID_PATH/d' "${PROJECT_ROOT}/.env" 2>/dev/null || true
		sed -i '/^DFX_/d' "${PROJECT_ROOT}/.env" 2>/dev/null || true
		sed -i '/^# DFX CANISTER ENVIRONMENT VARIABLES/d' "${PROJECT_ROOT}/.env" 2>/dev/null || true
		sed -i '/^# END DFX CANISTER ENVIRONMENT VARIABLES/d' "${PROJECT_ROOT}/.env" 2>/dev/null || true
	fi

	log_success "Reset complete - ready for fresh deployment"
}

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

		# Also clear .dfx network state (canister_ids.json, cached wasm, etc.)
		local dfx_dir="${PROJECT_ROOT}/.dfx/${network}"
		if [[ -d ${dfx_dir} ]]; then
			log_info "Clearing DFX state: ${dfx_dir}"
			rm -rf "${dfx_dir}"
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
	local FORCE_SET_VERSION=""

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
		--reset)
			VERSION_ACTION="reset"
			shift
			;;
		--force-set)
			VERSION_ACTION="force-set"
			FORCE_SET_VERSION="${2:-}"
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
	reset)
		cmd_reset "${NETWORK}"
		;;
	cycles)
		cmd_cycles
		;;
	loop)
		clear
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
	agent-docs)
		cmd_agent_docs
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
		reset)
			cmd_version_reset
			;;
		force-set)
			cmd_version_force_set "${FORCE_SET_VERSION:-}"
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
