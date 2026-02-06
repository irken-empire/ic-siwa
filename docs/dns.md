# DNS Configuration for IC Custom Domains

This document describes the DNS records required to configure custom domains for IC-SIWA frontend canisters.

## Overview

Custom domains for Internet Computer canisters require three DNS records. All records must have **Cloudflare Proxy OFF** (grey cloud / DNS only) because the IC boundary nodes handle TLS termination.

## Testnet Domain

**Domain:** `siwa-testnet.irkenempire.tech`
**Canister ID:** `kelzz-6qaaa-aaaak-qwlga-cai`

| Type  | Name                           | Value                                                   | Proxy |
| ----- | ------------------------------ | ------------------------------------------------------- | ----- |
| CNAME | `siwa-testnet`                 | `icp1.io`                                               | OFF   |
| TXT   | `_canister-id.siwa-testnet`    | `kelzz-6qaaa-aaaak-qwlga-cai`                           | -     |
| CNAME | `_acme-challenge.siwa-testnet` | `_acme-challenge.siwa-testnet.irkenempire.tech.icp2.io` | OFF   |

## Mainnet Domain

**Domain:** `siwa.irkenempire.tech`
**Canister ID:** TBD (update after mainnet deployment with debug=true)

| Type  | Name                   | Value                                           | Proxy |
| ----- | ---------------------- | ----------------------------------------------- | ----- |
| CNAME | `siwa`                 | `icp1.io`                                       | OFF   |
| TXT   | `_canister-id.siwa`    | `<canister-id>`                                 | -     |
| CNAME | `_acme-challenge.siwa` | `_acme-challenge.siwa.irkenempire.tech.icp2.io` | OFF   |

## Record Purposes

- **CNAME to icp1.io**: Routes traffic to IC boundary nodes
- **TXT \_canister-id**: Tells boundary nodes which canister to serve
- **CNAME \_acme-challenge**: Allows IC to provision SSL certificates automatically

## Verification

After adding DNS records, verify propagation:

```bash
# Check CNAME
dig +short CNAME siwa-testnet.irkenempire.tech

# Check TXT record
dig +short TXT _canister-id.siwa-testnet.irkenempire.tech

# Check ACME CNAME
dig +short CNAME _acme-challenge.siwa-testnet.irkenempire.tech
```

## References

- [IC Custom Domains Documentation](https://docs.internetcomputer.org/building-apps/frontends/custom-domains/using-custom-domains)
- [IC DNS Configuration Guide](https://docs.internetcomputer.org/building-apps/frontends/custom-domains/dns-setup)
