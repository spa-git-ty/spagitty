<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-035 — Native scrollbars, and a smudged tab

**Status:** Fixed.
**Branch:** `bugfix/BUG-035-native-scrollbars-and-a-smudged-tab`
**Screens:** all — every scrolling area on Windows; the tab strip.
**Raised by:** the author, with two screenshots from the Windows build: "shadow
on tab is wrong here, also the horizontal scroll with vertical is wrong."

## Problem

**Scrollbars.** `app.css` styles scrollbars twice: `scrollbar-width` and
`scrollbar-color` on every element, and `::-webkit-scrollbar` rules for a thin,
troughless, rounded thumb. WebKitGTK reads only the second. Chromium — WebView2,
on Windows — reads both, and when the standard properties are set they win, so
Windows drew the platform's own thin scrollbar: arrow buttons at each end, and
a square corner where a vertical and a horizontal bar meet, in the corner of a
rounded pane.

**The tab.** The open tab wore the ornaments' shadow, which is sized for an
object floating over the pane. On a pill in the title row it spread wider than
the pill and was cut off by the row, and read as a smudge.

## Scope

- The standard scrollbar properties only where `::-webkit-scrollbar` is not
  supported; the arrow buttons hidden where it is.
- The open tab has its surface and edge, and no shadow.

## Acceptance criteria

- On Windows, scrollbars are the theme's thin rounded thumbs with no arrows and
  no filled corner.
- The open tab has no shadow.
