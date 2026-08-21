#!/usr/bin/env python3

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

from ds_store import DSStore
from mac_alias import Alias

WINDOW_ORIGIN = (200, 120)
DEFAULT_WINDOW = (660, 400)
DEFAULT_ICON_SIZE = 128
DEFAULT_APP_XY = (180, 188)
DEFAULT_APPLICATIONS_XY = (480, 188)
HIDDEN_ILOC = (10000, 10000)
JUNK_NAMES = (".background", ".fseventsd", ".Trashes", ".Spotlight-V100")


def parse_pair(raw: str, label: str) -> tuple[int, int]:
    parts = raw.split(",")
    if len(parts) != 2:
        raise argparse.ArgumentTypeError(f"{label} must be X,Y")
    try:
        return int(parts[0], 10), int(parts[1], 10)
    except ValueError as error:
        raise argparse.ArgumentTypeError(f"{label} must be integers") from error


def parse_size(raw: str) -> tuple[int, int]:
    parts = raw.lower().split("x")
    if len(parts) != 2:
        raise argparse.ArgumentTypeError("window must be WIDTHxHEIGHT")
    try:
        width, height = int(parts[0], 10), int(parts[1], 10)
    except ValueError as error:
        raise argparse.ArgumentTypeError("window must be integers") from error
    if width <= 0 or height <= 0:
        raise argparse.ArgumentTypeError("window must be positive")
    return width, height


def require_mount_item(mount: Path, name: str, kind: str) -> Path:
    path = mount / name
    if kind == "dir" and not path.is_dir():
        raise SystemExit(f"DMG layout mount is missing directory {name}")
    if kind == "symlink" and not path.is_symlink():
        raise SystemExit(f"DMG layout mount is missing symlink {name}")
    if kind == "file" and not path.is_file():
        raise SystemExit(f"DMG layout mount is missing file {name}")
    return path


def write_layout(
    mount: Path,
    app_name: str,
    applications_name: str,
    background_relative: str,
    window: tuple[int, int],
    icon_size: int,
    app_xy: tuple[int, int],
    applications_xy: tuple[int, int],
) -> None:
    if sys.platform != "darwin":
        raise SystemExit("DMG layout writing requires Darwin getattrlist support")

    require_mount_item(mount, app_name, "dir")
    require_mount_item(mount, applications_name, "symlink")
    background = require_mount_item(mount, background_relative, "file")
    alias = Alias.for_file(os.fspath(background))
    left, top = WINDOW_ORIGIN
    ds_store_path = mount / ".DS_Store"
    visible = {app_name, applications_name}
    with DSStore.open(os.fspath(ds_store_path), "w+") as store:
        store["."]["vstl"] = ("type", "icnv")
        store["."]["bwsp"] = {
            "ShowTabView": False,
            "ShowToolbar": False,
            "ShowSidebar": False,
            "ShowPathbar": False,
            "ShowStatusBar": False,
            "SidebarWidth": 0,
            "ContainerShowSidebar": False,
            "PreviewPaneVisibility": False,
            # AppKit frame is {{x, y}, {width, height}}, not opposite-corner.
            "WindowBounds": "{{%d, %d}, {%d, %d}}"
            % (left, top, window[0], window[1]),
        }
        store["."]["icvp"] = {
            "viewOptionsVersion": 1,
            "backgroundType": 2,
            "backgroundColorRed": 1.0,
            "backgroundColorGreen": 1.0,
            "backgroundColorBlue": 1.0,
            "gridOffsetX": 0.0,
            "gridOffsetY": 0.0,
            "gridSpacing": 100.0,
            "arrangeBy": "none",
            "showIconPreview": False,
            "showItemInfo": False,
            "labelOnBottom": True,
            "textSize": 12.0,
            "iconSize": float(icon_size),
            "scrollPositionX": 0.0,
            "scrollPositionY": 0.0,
            "backgroundImageAlias": bytearray(alias.to_bytes()),
        }
        store[app_name]["Iloc"] = app_xy
        store[applications_name]["Iloc"] = applications_xy
        for name in JUNK_NAMES:
            store[name]["Iloc"] = HIDDEN_ILOC
        for entry in mount.iterdir():
            if entry.name in visible or entry.name == ".DS_Store":
                continue
            store[entry.name]["Iloc"] = HIDDEN_ILOC


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Write Finder-free .DS_Store layout onto a mounted UDRW DMG.",
    )
    parser.add_argument("--mount", required=True, type=Path)
    parser.add_argument("--app", default="FyAgent.app")
    parser.add_argument("--applications", default="Applications")
    parser.add_argument("--background", default=".background/background.png")
    parser.add_argument("--window", default="660x400", type=parse_size)
    parser.add_argument("--icon-size", default=DEFAULT_ICON_SIZE, type=int)
    parser.add_argument(
        "--app-xy",
        default="180,188",
        type=lambda value: parse_pair(value, "app-xy"),
    )
    parser.add_argument(
        "--apps-xy",
        default="480,188",
        type=lambda value: parse_pair(value, "apps-xy"),
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.icon_size <= 0:
        raise SystemExit("--icon-size must be positive")
    mount = args.mount.resolve()
    if not mount.is_dir():
        raise SystemExit(f"DMG layout mount is not a directory: {mount}")
    write_layout(
        mount,
        args.app,
        args.applications,
        args.background,
        args.window,
        args.icon_size,
        args.app_xy,
        args.apps_xy,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
