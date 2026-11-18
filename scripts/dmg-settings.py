# dmgbuild settings for Setter.dmg: the usual Mac disk image, with Setter on
# the left, an arrow, and Applications on the right to drag it onto.
#
#   dmgbuild -s scripts/dmg-settings.py -D app=path/to/Setter.app Setter Setter.dmg
#
# Used by the Mac job in .github/workflows/release.yml.
import os.path

application = defines["app"]  # noqa: F821
appname = os.path.basename(application)

format = "UDZO"
files = [application]
symlinks = {"Applications": "/Applications"}
hide_extensions = [appname]
# The disk's icon in Finder: the usual external disk with Setter's icon on it.
icon = os.path.join(application, "Contents", "Resources", "icon.icns")

background = "builtin-arrow"
window_rect = ((200, 120), (640, 280))
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
icon_size = 128
text_size = 14
icon_locations = {appname: (140, 120), "Applications": (500, 120)}
