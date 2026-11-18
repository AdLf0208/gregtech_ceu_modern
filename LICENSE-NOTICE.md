# Licence notice — GregTech CEu Modern for Horizon

This project is a **port of GregTech CEu Modern** to
[Horizon](https://github.com/horizon-rust/horizon), a Rust rewrite of Minecraft: Java
Edition. It is a **clean-room Rust rewrite**: no Java source was copied or translated,
and no part of `com.gregtechceu.gtceu`'s code is reproduced as Rust. It is an
**alteration of GregTech CEu Modern, not the mod itself**, and it is neither endorsed by
nor affiliated with the GregTech CEu developers.

## What this is ported from

- **GregTech CEu Modern**, mod version **7.0.2**, for Minecraft Java 1.21.1 (NeoForge).
- Upstream: <https://github.com/GregTechCEu/GregTech-Modern>, tag **`v7.0.2-1.21.1`**
  (commit `f6200e018582c3e26e8db06c289d9e01c8250ac8`).
- The jar this port was written against: **`gtceu-1.21.1-7.0.2.jar`**
  (29,257,797 B, 14,109 entries, SHA-256
  `c09a550523e931342a575193cbaad2e4b643e5de2cefe280cd1de540022c6388`), read from
  `ports/_shared/licence-cache/` in the campaign tree and kept there as evidence.
- The jar's own `META-INF/neoforge.mods.toml` declares
  `license = "LGPL-3.0 license"` and lists the authors, credited here in full:
  **KilaBash, screret, Mikerooni, serenibyss, Tech22, YoungOnion, Ghostipedia**
  (the field names Mikerooni twice). Its `credits` field credits **GregoriusT,
  Mr_Touchyou and Zerrens** for making GregTech and its textures, and the GT5u
  authors and maintainers for the original code.

## The licence, and what it obliges this port to do

GregTech CEu Modern is distributed under the **GNU Lesser General Public License,
version 3.0** (LGPL-3.0). The complete texts travel with this port:

- [`LICENSE`](LICENSE) — the **LGPL-3.0** text (GNU's `lgpl-3.0.txt`), and
- [`LICENSE-GPL-3.0`](LICENSE-GPL-3.0) — the **GPL-3.0** text, which the LGPL
  incorporates by reference (LGPL-3.0 §3).

This port is a **derivative work** of GregTech CEu Modern and is distributed under the
**same licence** (LGPL-3.0). Four obligations follow, and all four are met:

1. **The same licence, with both texts.** This project's licence is LGPL-3.0; the LGPL
   text and the GPL text it refers to are both in the repository and inside the delivered
   archive (`gtceu.hmod`, at the package root: `LICENSE`, `LICENSE-GPL-3.0` and
   `LICENSE-NOTICE.md` itself).
2. **The complete corresponding source travels with it.** The whole source of this port —
   the pure runtime (`src/runtime.rs`), the glue (`src/lib.rs`), `Cargo.toml`,
   `horizon-mod.toml`, and the content (`assets/gtceu/` and `data/`, with the generator
   `tools/assemble_content.py` in the repository) — is in this repository and inside the
   archive. Nothing of this port is closed.
3. **The modifications are stated** (LGPL-3.0 §5, "marked as changed"): this is a
   **rewrite against Horizon's named hooks**, not a copy. Every behaviour is
   re-implemented in Rust from the mod's own source read as documentation (the numbers
   and their `file:line` are in [`FINDINGS.md`](FINDINGS.md)); the block and item
   **models are rewritten** for Horizon's 26.3 model format (GT's `gtceu:machine` model
   loader and `neoforge:composite` ore model do not exist there — see
   [`CONTENT.md`](CONTENT.md)); the **textures, blockstates' rotations and language
   names are the jar's own files**, carried byte for byte where they are data.
4. **No further restriction is added** (LGPL-3.0 §3, §7): this project does not relicense
   GregTech CEu Modern, does not add terms to the LGPL, and does not restrict what the
   LGPL allows. A derivative of *this* port is bound by the same LGPL-3.0 and nothing
   more.

## What travels with the port

- [`LICENSE`](LICENSE) — LGPL-3.0, verbatim (165 lines, from
  <https://www.gnu.org/licenses/lgpl-3.0.txt>).
- [`LICENSE-GPL-3.0`](LICENSE-GPL-3.0) — GPL-3.0, verbatim (674 lines, from
  <https://www.gnu.org/licenses/gpl-3.0.txt>).
- This notice — in the repository **and inside the archive** (the package root).
- [`SOURCE.md`](SOURCE.md) — the jar, its SHA-256 and the upstream tag every carried
  asset came from; [`CONTENT.md`](CONTENT.md) lists each carried file and every rewrite
  of a model.
