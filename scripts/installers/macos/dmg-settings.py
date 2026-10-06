# dmgbuild settings for Baylee-<version>-<arch>.dmg (scripts/package-installers.sh).
#
# dmgbuild writes the window's .DS_Store itself, so no Finder and no
# AppleScript run: it works the same on a headless runner and locally.
# `defines` comes from `-D` on its command line:
#   app        the packaged Baylee.app (signed ad hoc by package-client.sh)
#   background background.png beside this file (dmgbuild picks up the @2x)
#   icon       the volume icon (Baylee.icns from the bundle)
# The icon positions are make-art.py's APP_X, APPS_X and ICON_Y.
# ruff: noqa: F821  (dmgbuild executes this file with `defines` in scope)

import os.path

app = defines["app"]
name = os.path.basename(app)

files = [app]
symlinks = {"Applications": "/Applications"}
icon = defines["icon"]
background = defines["background"]

# Read-only, LZFSE-compressed: macOS 10.11 and later, and the app needs 11.
format = "ULFO"
filesystem = "HFS+"

window_rect = ((200, 140), (660, 420))
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
show_icon_preview = False
include_icon_view_settings = True
include_list_view_settings = False

icon_size = 112
text_size = 13
arrange_by = None
icon_locations = {name: (170, 205), "Applications": (490, 205)}
