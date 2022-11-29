# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.0 (2022-11-29)

### Bug Fixes

<csr-id-a8a51b6efdb65b143df4a090cb250b4e546e105f/>

 - <csr-id-dd9e37c0a6d501df964495fff4435298df59cd95/> Update CHANGELOGs (start fresh)
 - <csr-id-a98c4d9d993b95cb9408faa24531d534733dbeb9/> Update nb to 1.0 and revert defmt-rtt to 0.3
   - defmt-rtt must be upgraded at the same time as defmt
- RUSTSEC-2020-0071 is from chrono which is used by probe-rs defmt
     dependencies (needs to be fixed upstream)
 - <csr-id-18a14cd0ff3ed3780c2857196c93d55bba025524/> Add CHANGELOGs

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 8 calendar days.
 - 11 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Add CHANGELOGs ([`18a14cd`](https://github.com/kiibohd/kiibohd-firmware/commit/18a14cd0ff3ed3780c2857196c93d55bba025524))
    - Update CHANGELOGs (start fresh) ([`dd9e37c`](https://github.com/kiibohd/kiibohd-firmware/commit/dd9e37c0a6d501df964495fff4435298df59cd95))
    - Update nb to 1.0 and revert defmt-rtt to 0.3 ([`a98c4d9`](https://github.com/kiibohd/kiibohd-firmware/commit/a98c4d9d993b95cb9408faa24531d534733dbeb9))
    - Fix defmt-rtt and ignore RUSTSEC-2020-0071 ([`a8a51b6`](https://github.com/kiibohd/kiibohd-firmware/commit/a8a51b6efdb65b143df4a090cb250b4e546e105f))
    - Adding more auditing tools ([`1b48ec1`](https://github.com/kiibohd/kiibohd-firmware/commit/1b48ec1a76075c3e897a83f82ee14d8df6ee730f))
</details>

<csr-unknown>
 Fix defmt-rtt and ignore RUSTSEC-2020-0071<csr-unknown/>

