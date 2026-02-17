#!/usr/bin/env bash

##################################################
# Name: codeql-run.sh
# Description: Local CodeQL static analysis wrapper
#
# Must run inside devenv shell (codeql is a devPackage).
#
# Usage:
#   codeql-run scan              Full scan (all languages)
#   codeql-run scan --lang rust  Scan single language
#   codeql-run db                Create/refresh databases
#   codeql-run analyze           Analyze existing databases
#   codeql-run results           View SARIF results summary
#   codeql-run clean             Remove databases and results
#   codeql-run help              Show usage
##################################################

clear
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Directories (gitignored)
DB_DIR="${PROJECT_ROOT}/.codeql-db"
RESULTS_DIR="${PROJECT_ROOT}/.codeql-results"

# Languages to scan (matching CI matrix, minus 'actions' which is GitHub-only)
ALL_LANGUAGES=("javascript-typescript" "rust")

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# ==========================================
# Preflight
# ==========================================

check_codeql() {
	if ! command -v codeql &>/dev/null; then
		log_error "codeql not found on PATH."
		log_error "Run this inside devenv shell: devenv shell"
		exit 1
	fi
}

# ==========================================
# Commands
# ==========================================

# Create CodeQL databases for specified languages
cmd_db() {
	local langs=("$@")
	[[ ${#langs[@]} -eq 0 ]] && langs=("${ALL_LANGUAGES[@]}")

	log_info "Creating CodeQL databases..."
	mkdir -p "${DB_DIR}"

	# Build CodeQL config argument if config file exists
	local config_args=()
	if [[ -f "${PROJECT_ROOT}/.github/codeql-config.yml" ]]; then
		log_info "Using config: .github/codeql-config.yml"
		# cspell:disable-next-line
		config_args=(--codescanning-config="${PROJECT_ROOT}/.github/codeql-config.yml")
	fi

	for lang in "${langs[@]}"; do
		local db_path="${DB_DIR}/${lang}"

		log_info "Creating database for ${lang}..."

		# Remove existing database to avoid stale state
		if [[ -d ${db_path} ]]; then
			log_warn "Removing existing database: ${db_path}"
			rm -rf "${db_path}"
		fi

		codeql database create "${db_path}" \
			--language="${lang}" \
			--source-root="${PROJECT_ROOT}" \
			"${config_args[@]}" \
			--overwrite \
			2>&1 | tail -5

		log_success "Database created: ${db_path}"
	done
}

# Analyze existing databases
cmd_analyze() {
	local langs=("$@")
	[[ ${#langs[@]} -eq 0 ]] && langs=("${ALL_LANGUAGES[@]}")

	log_info "Analyzing CodeQL databases..."
	mkdir -p "${RESULTS_DIR}"

	for lang in "${langs[@]}"; do
		local db_path="${DB_DIR}/${lang}"
		local sarif_file="${RESULTS_DIR}/${lang}.sarif"
		local csv_file="${RESULTS_DIR}/${lang}.csv"

		if [[ ! -d ${db_path} ]]; then
			log_warn "No database for ${lang} — run 'codeql-run db' first. Skipping."
			continue
		fi

		log_info "Analyzing ${lang}..."

		# Run the default security-and-quality suite
		codeql database analyze "${db_path}" \
			--format=sarifv2.1.0 \
			--output="${sarif_file}" \
			--sarif-add-query-help \
			2>&1 | tail -10

		log_success "SARIF results: ${sarif_file}"

		# Also generate CSV for quick viewing
		# cspell:disable-next-line
		if codeql database interpret-results "${db_path}" \
			--format=csv \
			--output="${csv_file}" \
			2>/dev/null; then
			log_success "CSV results:   ${csv_file}"
		fi
	done
}

# Full scan: create DB + analyze
cmd_scan() {
	local langs=("$@")
	[[ ${#langs[@]} -eq 0 ]] && langs=("${ALL_LANGUAGES[@]}")

	local start_time
	start_time=$(date +%s)

	log_info "Starting full CodeQL scan..."
	echo ""

	cmd_db "${langs[@]}"
	echo ""
	cmd_analyze "${langs[@]}"
	echo ""

	local end_time
	end_time=$(date +%s)
	local elapsed=$((end_time - start_time))
	local minutes=$((elapsed / 60))
	local seconds=$((elapsed % 60))

	log_success "Scan complete in ${minutes}m ${seconds}s"
	echo ""
	cmd_results
}

# Show results summary
cmd_results() {
	if [[ ! -d ${RESULTS_DIR} ]]; then
		log_warn "No results found. Run 'codeql-run scan' first."
		return 0
	fi

	local found_results=false

	for sarif_file in "${RESULTS_DIR}"/*.sarif; do
		[[ -f ${sarif_file} ]] || continue
		found_results=true

		local lang
		lang=$(basename "${sarif_file}" .sarif)
		local count

		# Count results from SARIF (jq required)
		if command -v jq &>/dev/null; then
			count=$(jq '[.runs[].results[]] | length' "${sarif_file}" 2>/dev/null || echo "?")
		else
			count="(install jq for count)"
		fi

		if [[ ${count} == "0" ]]; then
			echo -e "  ${GREEN}✓${NC} ${lang}: ${count} findings"
		else
			echo -e "  ${YELLOW}⚠${NC} ${lang}: ${count} findings"
		fi
	done

	if [[ ${found_results} == "false" ]]; then
		log_warn "No SARIF results found. Run 'codeql-run scan' first."
		return 0
	fi

	echo ""
	log_info "SARIF files in: ${RESULTS_DIR}/"
	log_info "Open .sarif files in VS Code (SARIF Viewer extension) for detailed review."
}

# Clean up databases and results
cmd_clean() {
	log_info "Cleaning CodeQL artifacts..."

	if [[ -d ${DB_DIR} ]]; then
		rm -rf "${DB_DIR}"
		log_success "Removed ${DB_DIR}"
	fi

	if [[ -d ${RESULTS_DIR} ]]; then
		rm -rf "${RESULTS_DIR}"
		log_success "Removed ${RESULTS_DIR}"
	fi

	log_success "Clean complete"
}

# Show help
cmd_help() {
	cat <<-EOF
		CodeQL Local Scanner

		Usage: codeql-run <command> [options]

		Commands:
		  scan              Full scan (create DB + analyze + results)
		  db                Create/refresh CodeQL databases
		  analyze           Analyze existing databases
		  results           View results summary
		  clean             Remove databases and results
		  help              Show this help

		Options:
		  --lang <language> Scan a single language only
		                    Available: ${ALL_LANGUAGES[*]}

		Examples:
		  codeql-run scan                    # Scan all languages
		  codeql-run scan --lang rust        # Scan Rust only
		  codeql-run scan --lang javascript-typescript
		  codeql-run results                 # View findings
		  codeql-run clean                   # Remove all artifacts

		Environment:
		  Must run inside devenv shell (codeql is a devPackage).
		  Database:  .codeql-db/   (gitignored)
		  Results:   .codeql-results/  (gitignored)
	EOF
}

# ==========================================
# Main
# ==========================================

main() {
	check_codeql

	local command="${1:-help}"
	shift || true

	# Parse --lang flag
	local selected_langs=()
	while [[ $# -gt 0 ]]; do
		case "$1" in
		--lang | -l)
			shift
			if [[ $# -eq 0 ]]; then
				log_error "--lang requires a language argument"
				exit 1
			fi
			selected_langs+=("$1")
			;;
		*)
			log_error "Unknown argument: $1"
			cmd_help
			exit 1
			;;
		esac
		shift
	done

	case "${command}" in
	scan) cmd_scan "${selected_langs[@]}" ;;
	db) cmd_db "${selected_langs[@]}" ;;
	analyze) cmd_analyze "${selected_langs[@]}" ;;
	results) cmd_results ;;
	clean) cmd_clean ;;
	help | --help | -h) cmd_help ;;
	*)
		log_error "Unknown command: ${command}"
		cmd_help
		exit 1
		;;
	esac
}

main "$@"
