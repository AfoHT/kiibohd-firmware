# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 (2022-11-17)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 40 commits contributed to the release over the course of 593 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#6](https://github.com/kiibohd/kiibohd-firmware/issues/6)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#6](https://github.com/kiibohd/kiibohd-firmware/issues/6)**
    - Added reading project.env to build.rs for VID and PID ([`32592dd`](https://github.com/kiibohd/kiibohd-firmware/commit/32592dd350a634708040d3cd2144fcbac766e900))
 * **Uncategorized**
    - Add initial CHANGELOG.md files ([`cd36b7e`](https://github.com/kiibohd/kiibohd-firmware/commit/cd36b7ed6e28b1172afe4b5b05204e30c5f5640d))
    - Add firmware revision support for bootloader ([`370cac8`](https://github.com/kiibohd/kiibohd-firmware/commit/370cac807f17e8ab407a93c83e9128be9018400b))
    - Large refactor ([`2f4d804`](https://github.com/kiibohd/kiibohd-firmware/commit/2f4d804443941f429eae191a022bc40c55b188fe))
    - Updating versions, clippy fix and update ISSI app note comment ([`964bf66`](https://github.com/kiibohd/kiibohd-firmware/commit/964bf667342744649b6ac68149f8ef619a123994))
    - Fix general clippy errors ([`7dfcb88`](https://github.com/kiibohd/kiibohd-firmware/commit/7dfcb88ae2233ad2091e3278376ef3ffb85dca67))
    - Fix bootloader offset ([`b20c0b7`](https://github.com/kiibohd/kiibohd-firmware/commit/b20c0b76899a78c419ed0aa6416930ecae9615cb))
    - Cleanup configs ([`7d3c85c`](https://github.com/kiibohd/kiibohd-firmware/commit/7d3c85c2c5d4e5cb631366cf03af7195d8b9be4f))
    - Add support for DWT Tickless Monotonics in rtic ([`7b1a3be`](https://github.com/kiibohd/kiibohd-firmware/commit/7b1a3be7c3ba1a05de04e56e3600ec7d2687c444))
    - [Gemini] Split bin.rs to constants and hidio ([`2227d61`](https://github.com/kiibohd/kiibohd-firmware/commit/2227d617559839e682fa2361ff136010a832ea13))
    - Initial manufacturing test HID-IO message handling ([`e204b17`](https://github.com/kiibohd/kiibohd-firmware/commit/e204b171652dae9eb7a7568877a4f0736d9511af))
    - Adding USB HID Lock LED KLL support ([`cf754d6`](https://github.com/kiibohd/kiibohd-firmware/commit/cf754d67d1c866dab5853884beed97690cccac9e))
    - [Gemini] USB now working ([`6381faf`](https://github.com/kiibohd/kiibohd-firmware/commit/6381fafc6847c0394325f2d502ea2fa23752e95c))
    - Fix/enable USB HID Mouse support ([`ae3aa1e`](https://github.com/kiibohd/kiibohd-firmware/commit/ae3aa1e087bc3f6cfbb5ad3683b7057c8144dfc6))
    - Updating Cargo.toml to use top-level override ([`15b9f59`](https://github.com/kiibohd/kiibohd-firmware/commit/15b9f591b7fa0ddd1a9937b8efe815cc8e09111c))
    - Move some configurations from .cargo/config to common_makefile.toml ([`4a30fb3`](https://github.com/kiibohd/kiibohd-firmware/commit/4a30fb3be3801250e5ff0d3df6a6e5da6da25d4c))
    - Update vergen to use official version ([`2ba90f9`](https://github.com/kiibohd/kiibohd-firmware/commit/2ba90f9b43e252f6f8df809477316397b2e3b2aa))
    - Integrate keyscanning to kll-core to USB output ([`fd69982`](https://github.com/kiibohd/kiibohd-firmware/commit/fd699820b716afb1aa4f9c8f60fbe7a607003b12))
    - Add stack pointer debugging ([`9755e39`](https://github.com/kiibohd/kiibohd-firmware/commit/9755e39840fc0869d2431a95884e7bab4c199275))
    - Add device mcu and device serial number to HID-IO info ([`3366b55`](https://github.com/kiibohd/kiibohd-firmware/commit/3366b55b1cf8f4da274d88a0e0dd211dac8794fa))
    - Add bcdDevice version using git commit count ([`756c6be`](https://github.com/kiibohd/kiibohd-firmware/commit/756c6be59cd8c005156846f360bff0f8ab777857))
    - More more variables to project.env files ([`93648e1`](https://github.com/kiibohd/kiibohd-firmware/commit/93648e133fb06ac3db2219e093926f4b66cda286))
    - Update rtic to 1.0.0 ([`00ec115`](https://github.com/kiibohd/kiibohd-firmware/commit/00ec1158b39988b181a74d44910f857fa9664ba2))
    - Adding git submodule support to GitHub Actions ([`725197f`](https://github.com/kiibohd/kiibohd-firmware/commit/725197f696f3263038ebbebac8eca9db2c1d9dc9))
    - [KLL] Updating Gemini to use the new build.rs format ([`7297612`](https://github.com/kiibohd/kiibohd-firmware/commit/72976125834fd0e68292d3756b97ba5c98231dc8))
    - Fixing clippy warnings and adding host_deps option ([`7bb7c43`](https://github.com/kiibohd/kiibohd-firmware/commit/7bb7c43db1c447d60bb39f5b0a4eb8298a5ada40))
    - Clippy fix ([`5bcf6b5`](https://github.com/kiibohd/kiibohd-firmware/commit/5bcf6b5b9ab917f6ea536f810fcce348f0427c94))
    - Updating defmt to 0.3 ([`a17dffe`](https://github.com/kiibohd/kiibohd-firmware/commit/a17dffe1324eaaf98d9f7d8692dbd8e73aeacdc3))
    - Updating to cortex-m-rtic 0.6.0-rc4 ([`43d2619`](https://github.com/kiibohd/kiibohd-firmware/commit/43d2619338b4012ae625fc0fc06b97515b5fdf39))
    - Adding EFC support to retrieve UID ([`071b7d9`](https://github.com/kiibohd/kiibohd-firmware/commit/071b7d91dbd8d6c8224a4c63eead9ff7c04a5a3d))
    - Adding gdb+defmt support workflows ([`7ce364e`](https://github.com/kiibohd/kiibohd-firmware/commit/7ce364e0eea8665ea2f082bb43b37045f52374bd))
    - Updating to 2021 edition ([`85613bc`](https://github.com/kiibohd/kiibohd-firmware/commit/85613bc7007c3af6ff4fe334aace1b13a2fc92dc))
    - Re-organized cargo make ([`ebe5292`](https://github.com/kiibohd/kiibohd-firmware/commit/ebe52929260abd797d51b3c15aca139042995ba7))
    - Keyboards are starting to take shape ([`c236166`](https://github.com/kiibohd/kiibohd-firmware/commit/c2361665d05318ca377862eb67fefa56c43bf023))
    - Initial UDP integration ([`d71a47d`](https://github.com/kiibohd/kiibohd-firmware/commit/d71a47d7837f09c0b1145f37d477f3c476ba778f))
    - Moving top-level to rtic ([`944cc4e`](https://github.com/kiibohd/kiibohd-firmware/commit/944cc4e7c7db565b4063348b161502cd9611dbee))
    - Merge pull request #4 from hiszd/main ([`00ddd2c`](https://github.com/kiibohd/kiibohd-firmware/commit/00ddd2c0b6627b13a2b4f62d3ce43678a9c5933e))
    - Adding GitHub Action tests ([`b4004eb`](https://github.com/kiibohd/kiibohd-firmware/commit/b4004eb85b93c502cdf71c38227a0f4126995825))
    - Adding build environment variables for KLL files ([`680af1a`](https://github.com/kiibohd/kiibohd-firmware/commit/680af1ac9bdbbdc1000acc2381a513031e6dcfb1))
    - Adding initial Hexgears Gemini Dusk/Dawn skeleton ([`2b6909c`](https://github.com/kiibohd/kiibohd-firmware/commit/2b6909c98bb6eb9e21ba13ea45379c891bf87064))
</details>

