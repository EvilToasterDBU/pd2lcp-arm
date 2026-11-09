# Russian S14 native HD menu text

The PD2 fork of D2GL replaces menu DC6 images with real HD text in `HDText::drawImage`. It recognizes their frame count and last-frame dimensions. The English table includes PD2 controls; the Russian table contains only vanilla controls. Repainting or resizing localized DC6s breaks this recognition.

`apply_translation.py` adds all 80 PD2 entries to the Russian table, using the original renderer's font selection, colors, shadows and alignment. It preserves the existing vanilla Russian entries. Two ambiguous signatures (SkillBar and SNext) receive distinct transparent canvas widths, without repainting glyphs.

The build workflow pins Project-Diablo-2/d2gl commit 55148de0aec79d487f5bcef1842cfdc3effd2f62 and builds Win32 Glide/DDraw wrappers with MSVC. These are x86 DLLs for the x86 game even on ARM Linux.

Prepare the loose Russian UI fallback files from the installed S14 archive:

```
python3 prepare_ui.py /path/pd2assets.mpq /path/listfile /path/mpq-tool /path/output
```

Install the built glide3x.dll/ddraw.dll into ProjectD2 and the prepared DC6 files into ProjectD2/data/local/ui/rus, backing up existing files. Keep the installed d2gl.mpq and its HD font atlases. Do not regenerate these UI images with the obsolete bitmap-label generator.
