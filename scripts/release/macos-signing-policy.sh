#!/usr/bin/env bash
# Public Developer ID policy for FyAgent macOS release artifacts.
# This file is sourced by signing and verification helpers; it is not a secret.

EXPECTED_IDENTIFIER='com.fyagent.desktop'
EXPECTED_TEAM_ID='HY446996QX'
EXPECTED_AUTHORITY='Developer ID Application: William Wang (HY446996QX)'
EXPECTED_BUNDLE_NAME='FyAgent.app'
EXPECTED_HELPER_IDENTIFIER='com.fyagent.desktop.system-commit-helper'
EXPECTED_PRIVILEGED_HELPER_RELPATH='Contents/Library/LaunchServices/com.fyagent.desktop.system-commit-helper'
EXPECTED_PRIVILEGED_CLIENT_RELPATH='Contents/Frameworks/libFyAgentPrivilegedClient.dylib'
NOTARY_PROFILE='fyagent-notary'
