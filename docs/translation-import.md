# Importing a translation

Quran Reader ships only translations that are verifiably free to redistribute (see
[`LICENSES/README.md`](../LICENSES/README.md)). Any other translation — for example the
ones on [tanzil.net/trans](https://tanzil.net/trans/), which may be used for non-commercial
purposes only — you can download yourself and import:

**Reader settings (⌘,) → Translation → Import…**

The file is checked, converted and stored in the app's data folder on your computer
(`~/Library/Application Support/io.github.quranreader.desktop/translations/` on macOS,
`%APPDATA%\io.github.quranreader.desktop\translations\` on Windows,
`~/.local/share/io.github.quranreader.desktop/translations/` on Linux). It is never uploaded,
shared or bundled with the app. Importing a file with the same id again replaces it; an
imported translation can be removed from the same place in the settings.

An imported translation shows in the stacked and two-column views, in the compare view (C)
and in search (⌘F) like a bundled one.

## Formats

UTF-8 text in one of Tanzil's download formats. The format is detected from the content.

### Plain text (Tanzil "Text")

One verse per line, all 6,236 verses in Quran order (Hafs, Kufan count — the numbering of
the Madina Mushaf and of Tanzil). Lines starting with `#` are comments; blank lines at the
end are ignored.

```text
<text of 1:1>
<text of 1:2>
…
<text of 114:6>

#  Name: My Translation
#  Translator: …
#  Language: English
#  ID: en.mine
```

### Text with verse numbers (Tanzil "Text with aya numbers")

`surah|ayah|text` per line, in any order. `#` comments as above.

```text
1|1|<text of 1:1>
1|2|<text of 1:2>
```

### XML (Tanzil "XML")

```xml
<?xml version="1.0" encoding="utf-8" ?>
<!--
  Name: My Translation
  Translator: …
  Language: English
  ID: en.mine
-->
<quran>
  <sura index="1" name="…">
    <aya index="1" text="&lt;text of 1:1&gt;" />
  </sura>
</quran>
```

## What is checked

- Every one of the 6,236 verses is there exactly once. A plain-text file must have exactly
  6,236 verse lines; a numbered or XML file must not skip, repeat or invent a verse
  (`2:287` does not exist). Otherwise the import is refused with the line that is wrong, so
  a translation is never shown next to the wrong verse.
- Verse texts are stored exactly as in the file; nothing is reworded or reformatted.

## Description

The comments (Tanzil puts its copyright and terms there) are kept and shown with the
translation. These comment fields are used when present:

| Field | Used for |
| --- | --- |
| `Name:` | the name in the settings; otherwise the file name |
| `Translator:` | shown in the compare view and the credits |
| `ID:` | the id (`user.<ID>`); its prefix (`de.`, `en.`) is the language |
| `Language:` | the language, when there is no `ID:` (English names such as `German`) |

Without either, the language is "Other".
