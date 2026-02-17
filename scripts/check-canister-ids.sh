#!/usr/bin/env bash
# Validates that canister IDs in YAML configs match canister_ids.*.json files.
# Single source of truth: canister_ids.{mainnet,testnet}.json (DFX registry)
# YAML configs must mirror these values exactly.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

ERRORS=0

check_match() {
	local env="$1"
	local yaml_file="${PROJECT_ROOT}/config/${env}.yaml"
	local json_file="${PROJECT_ROOT}/canister_ids.${env}.json"

	if [[ ! -f ${yaml_file} ]]; then
		echo "SKIP: ${yaml_file} not found"
		return
	fi
	if [[ ! -f ${json_file} ]]; then
		echo "SKIP: ${json_file} not found"
		return
	fi

	echo "Checking ${env}..."

	# Extract canister IDs from JSON (format: "name": { "ic": "id" })
	local canisters
	canisters=$(jq -r 'to_entries[] | "\(.key)=\(.value.ic)"' "${json_file}")

	for entry in ${canisters}; do
		local name="${entry%%=*}"
		local json_id="${entry#*=}"

		# Look up the same canister in YAML
		local yaml_id
		yaml_id=$(yq -r ".ic.canisters.${name} // \"\"" "${yaml_file}")

		if [[ -z ${yaml_id} ]]; then
			echo "  WARN: ${name} in ${json_file##*/} but not in ${yaml_file##*/}"
			continue
		fi

		if [[ ${json_id} != "${yaml_id}" ]]; then
			echo "  ERROR: ${name} mismatch"
			echo "    ${json_file##*/}: ${json_id}"
			echo "    ${yaml_file##*/}: ${yaml_id}"
			ERRORS=$((ERRORS + 1))
		else
			echo "  OK: ${name} = ${json_id}"
		fi
	done
}

check_match "mainnet"
check_match "testnet"

if [[ ${ERRORS} -gt 0 ]]; then
	echo ""
	echo "FAILED: ${ERRORS} canister ID mismatch(es) found."
	echo "The single source of truth is canister_ids.{mainnet,testnet}.json."
	echo "Update the YAML configs to match."
	exit 1
fi

echo ""
echo "All canister IDs are consistent."
