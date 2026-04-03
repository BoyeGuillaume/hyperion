#!/bin/bash

FILE_PATH="$(dirname "$(realpath "$0")")"
ASSETS_PATH="$FILE_PATH/../assets"

magick -background transparent \
    -define 'icon:auto-resize=64,32,16' \
    "$ASSETS_PATH/hyperion-icon.svg" "$ASSETS_PATH/favicon.ico"