#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const ROOT = path.resolve(path.dirname(SCRIPT_PATH), "..", "..");
const TEAM_ID = "HY446996QX";
const DEV_APP_IDENTIFIER = "com.fyagent.desktop.dev";
const DEV_HELPER_IDENTIFIER = "com.fyagent.desktop.dev.system-commit-helper";
const DEV_CLIENT_IDENTIFIER = "com.fyagent.desktop.dev.privileged-client";
const CLIENT_FILE = "libFyAgentPrivilegedClient.dylib";
const ARTIFACT_SCHEMA = "fyagent-macos-privileged-artifacts/v1";
const XCODE_DEVELOPER_DIR = "/Applications/Xcode.app/Contents/Developer";
const DEVELOPER_ID_AUTHORITY =
  "Developer ID Application: William Wang (HY446996QX)";
const SIGNING_CONFIG_SCHEMA = "fyagent-macos-signed-dev-signing/v1";
const SIGNING_CONFIG_PATH = path.join(
  os.homedir(),
  "Library",
  "Application Support",
  "FyAgent",
  "DevelopmentSigning",
  "config.json",
);
const SIGNING_CACHE = path.join(
  os.homedir(),
  "Library",
  "Caches",
  "FyAgent",
  "DevelopmentSigning",
);
const SESSION_KEYCHAIN = path.join(SIGNING_CACHE, "signing.keychain-db");
const SESSION_KEYCHAIN_PASSWORD = path.join(
  SIGNING_CACHE,
  "keychain.password",
);
const DEVELOPER_ID_INTERMEDIATE = path.join(
  ROOT,
  "scripts",
  "release",
  "apple-developer-id-g2-ca.cer",
);
const APPLE_ROOT_CA = path.join(
  ROOT,
  "scripts",
  "release",
  "apple-root-ca.cer",
);

function fail(message) {
  throw new Error(`macos-signed-dev: ${message}`);
}

function run(command, args = [], options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? ROOT,
    env: { ...process.env, ...(options.env ?? {}) },
    encoding: "utf8",
    stdio: options.inherit ? "inherit" : "pipe",
    windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    const detail = `${result.stderr ?? ""}${result.stdout ?? ""}`.trim();
    fail(
      `${path.basename(command)} exited with ${result.status}${detail ? `: ${detail}` : ""}`,
    );
  }
  return {
    stdout: (result.stdout ?? "").trim(),
    stderr: (result.stderr ?? "").trim(),
  };
}

function regularFile(file, label) {
  const absolute = path.resolve(file);
  let stat;
  try {
    stat = fs.lstatSync(absolute);
  } catch {
    fail(`${label} does not exist: ${absolute}`);
  }
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size <= 0) {
    fail(`${label} must be a non-empty regular non-symlink file: ${absolute}`);
  }
  return absolute;
}

function regularDirectory(directory, label) {
  const absolute = path.resolve(directory);
  let stat;
  try {
    stat = fs.lstatSync(absolute);
  } catch {
    fail(`${label} does not exist: ${absolute}`);
  }
  if (!stat.isDirectory() || stat.isSymbolicLink()) {
    fail(`${label} must be a regular non-symlink directory: ${absolute}`);
  }
  return absolute;
}

function readJson(file, label) {
  const absolute = regularFile(file, label);
  try {
    return { absolute, value: JSON.parse(fs.readFileSync(absolute, "utf8")) };
  } catch (error) {
    fail(
      `${label} is not valid JSON: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

function readCargoVersion() {
  const cargo = fs.readFileSync(
    path.join(ROOT, "src-tauri", "Cargo.toml"),
    "utf8",
  );
  const marker = "[workspace.package]";
  const start = cargo.indexOf(marker);
  if (start < 0) fail("workspace package table is missing");
  const remainder = cargo.slice(start + marker.length);
  const nextTable = /^\[/m.exec(remainder)?.index ?? remainder.length;
  const workspace = remainder.slice(0, nextTable);
  const version = /^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"\s*$/m.exec(
    workspace,
  )?.[1];
  if (!version) fail("workspace package version is missing or invalid");
  return version;
}

function xml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

function assertFullXcode() {
  const configured = process.env.DEVELOPER_DIR;
  if (configured && configured !== XCODE_DEVELOPER_DIR) {
    fail(
      `DEVELOPER_DIR must be the project-owned full Xcode path ${XCODE_DEVELOPER_DIR}`,
    );
  }
  regularDirectory(XCODE_DEVELOPER_DIR, "full Xcode developer directory");
  const result = run("/usr/bin/xcodebuild", ["-version"], {
    env: { DEVELOPER_DIR: XCODE_DEVELOPER_DIR },
  });
  if (!/^Xcode\s+\S+/m.test(result.stdout)) {
    fail("xcodebuild did not report a full Xcode installation");
  }
  run("/usr/bin/xcodebuild", ["-checkFirstLaunchStatus"], {
    env: { DEVELOPER_DIR: XCODE_DEVELOPER_DIR },
  });
}

function parseSigningCredentials(file) {
  const source = fs.readFileSync(
    regularFile(file, "signing credentials file"),
    "utf8",
  );
  const values = new Map();
  for (const rawLine of source.split(/\r?\n/)) {
    const separator = rawLine.indexOf(":");
    if (separator <= 0) continue;
    const key = rawLine.slice(0, separator).trim();
    const value = rawLine.slice(separator + 1).trim();
    if (!key || !value) continue;
    if (values.has(key)) fail(`signing credentials contain duplicate ${key}`);
    values.set(key, value);
  }
  const password = values.get("p12_password");
  if (!password) fail("signing credentials do not contain p12_password");
  const configuredTeam = values.get("apple_team_id");
  if (configuredTeam && configuredTeam !== TEAM_ID) {
    fail("signing credentials Apple team does not match FyAgent");
  }
  return { password };
}

function loadSigningConfig() {
  const { value } = readJson(
    SIGNING_CONFIG_PATH,
    "signed development configuration",
  );
  if (
    value?.schema !== SIGNING_CONFIG_SCHEMA ||
    value?.teamIdentifier !== TEAM_ID ||
    typeof value?.developerIdP12 !== "string" ||
    typeof value?.credentialsFile !== "string"
  ) {
    fail("signed development configuration has an unsupported schema");
  }
  return {
    developerIdP12: regularFile(value.developerIdP12, "Developer ID PKCS#12"),
    credentialsFile: regularFile(
      value.credentialsFile,
      "signing credentials file",
    ),
  };
}

function parseDeveloperIdIdentity(output) {
  const identities = [];
  for (const line of String(output).split("\n")) {
    const match = /^\s*\d+\)\s+([0-9A-Fa-f]{40})\s+"([^"]+)"/.exec(line);
    if (!match || match[2] !== DEVELOPER_ID_AUTHORITY) continue;
    identities.push({ hash: match[1].toUpperCase(), authority: match[2] });
  }
  if (identities.length !== 1) {
    fail(
      `temporary keychain must expose exactly one ${DEVELOPER_ID_AUTHORITY} identity; found ${identities.length}`,
    );
  }
  return identities[0];
}

function captureUserKeychains() {
  const originalKeychains = run("/usr/bin/security", [
    "list-keychains",
    "-d",
    "user",
  ])
    .stdout.split(/\r?\n/)
    .map((value) => value.trim().replace(/^"|"$/g, ""))
    .filter(Boolean)
    .filter((value) => value !== SESSION_KEYCHAIN);
  let originalDefaultKeychain = run("/usr/bin/security", [
    "default-keychain",
    "-d",
    "user",
  ])
    .stdout.trim()
    .replace(/^"|"$/g, "");
  if (!originalDefaultKeychain || originalDefaultKeychain === SESSION_KEYCHAIN) {
    originalDefaultKeychain =
      originalKeychains[0] ??
      path.join(os.homedir(), "Library", "Keychains", "login.keychain-db");
  }
  if (!originalDefaultKeychain) {
    fail("user default keychain could not be resolved");
  }
  return {
    originalKeychains,
    originalDefaultKeychain,
    restoreKeychains:
      originalKeychains.length > 0
        ? originalKeychains
        : [originalDefaultKeychain],
  };
}

function restoreUserKeychains(session) {
  spawnSync(
    "/usr/bin/security",
    ["default-keychain", "-d", "user", "-s", session.originalDefaultKeychain],
    { stdio: "ignore", windowsHide: true },
  );
  spawnSync(
    "/usr/bin/security",
    ["list-keychains", "-d", "user", "-s", ...session.restoreKeychains],
    { stdio: "ignore", windowsHide: true },
  );
}

function sessionKeychainPassword() {
  if (fs.existsSync(SESSION_KEYCHAIN_PASSWORD)) {
    const value = fs.readFileSync(SESSION_KEYCHAIN_PASSWORD, "utf8").trim();
    if (!value) fail("cached signing keychain password file is empty");
    return value;
  }
  const password = crypto.randomBytes(32).toString("base64url");
  fs.writeFileSync(SESSION_KEYCHAIN_PASSWORD, `${password}\n`, { mode: 0o600 });
  fs.chmodSync(SESSION_KEYCHAIN_PASSWORD, 0o600);
  return password;
}

function unlockSessionKeychain(keychain, password) {
  const unlocked = spawnSync(
    "/usr/bin/security",
    ["unlock-keychain", "-p", password, keychain],
    { encoding: "utf8", windowsHide: true },
  );
  if (unlocked.status !== 0) return false;
  run("/usr/bin/security", [
    "set-keychain-settings",
    "-lut",
    "21600",
    keychain,
  ]);
  return true;
}

function importDeveloperIdMaterial(keychain, password, config, p12Password) {
  const appleRoot = regularFile(APPLE_ROOT_CA, "Apple Root CA certificate");
  const intermediate = regularFile(
    DEVELOPER_ID_INTERMEDIATE,
    "Apple Developer ID G2 intermediate certificate",
  );
  const extract = fs.mkdtempSync(path.join(SIGNING_CACHE, "extract-"));
  fs.chmodSync(extract, 0o700);
  const leafCertificate = path.join(extract, "developer-id-leaf.pem");
  const extractedPrivateKey = path.join(extract, "developer-id-key.pem");
  const rsaPrivateKey = path.join(extract, "developer-id-rsa.pem");
  try {
    const opensslEnvironment = {
      FYAGENT_SIGNED_DEV_P12_PASSWORD: p12Password,
    };
    // macOS 26 can import this legacy RC2/3DES PKCS#12 without error while
    // still failing to expose a usable SecIdentity to codesign. Re-express
    // the same certificate/key pair as PEM and import those items separately.
    // Extracted PEM lives only in this 0700 directory and is deleted after
    // import; the session keychain keeps the imported SecIdentity.
    run(
      "/usr/bin/openssl",
      [
        "pkcs12",
        "-in",
        config.developerIdP12,
        "-passin",
        "env:FYAGENT_SIGNED_DEV_P12_PASSWORD",
        "-clcerts",
        "-nokeys",
        "-out",
        leafCertificate,
      ],
      { env: opensslEnvironment },
    );
    run(
      "/usr/bin/openssl",
      [
        "pkcs12",
        "-in",
        config.developerIdP12,
        "-passin",
        "env:FYAGENT_SIGNED_DEV_P12_PASSWORD",
        "-nocerts",
        "-nodes",
        "-out",
        extractedPrivateKey,
      ],
      { env: opensslEnvironment },
    );
    run("/usr/bin/openssl", [
      "rsa",
      "-in",
      extractedPrivateKey,
      "-out",
      rsaPrivateKey,
    ]);
    for (const sensitiveFile of [
      leafCertificate,
      extractedPrivateKey,
      rsaPrivateKey,
    ]) {
      fs.chmodSync(
        regularFile(sensitiveFile, "temporary signing material"),
        0o600,
      );
    }
    run("/usr/bin/security", [
      "import",
      appleRoot,
      "-k",
      keychain,
      "-T",
      "/usr/bin/codesign",
      "-T",
      "/usr/bin/security",
    ]);
    run("/usr/bin/security", [
      "import",
      intermediate,
      "-k",
      keychain,
      "-T",
      "/usr/bin/codesign",
      "-T",
      "/usr/bin/security",
    ]);
    run("/usr/bin/security", [
      "import",
      leafCertificate,
      "-k",
      keychain,
      "-t",
      "cert",
      "-f",
      "pemseq",
    ]);
    run("/usr/bin/security", [
      "import",
      rsaPrivateKey,
      "-k",
      keychain,
      "-t",
      "priv",
      "-f",
      "openssl",
      "-T",
      "/usr/bin/codesign",
      "-T",
      "/usr/bin/security",
    ]);
    run("/usr/bin/security", [
      "set-key-partition-list",
      "-S",
      "apple-tool:,apple:,codesign:",
      "-s",
      "-k",
      password,
      keychain,
    ]);
    run("/usr/bin/security", ["unlock-keychain", "-p", password, keychain]);
  } finally {
    fs.rmSync(extract, { recursive: true, force: true });
  }
}

function activateSessionKeychain(keychain, session) {
  run("/usr/bin/security", [
    "list-keychains",
    "-d",
    "user",
    "-s",
    keychain,
    ...session.originalKeychains,
  ]);
  run("/usr/bin/security", [
    "default-keychain",
    "-d",
    "user",
    "-s",
    keychain,
  ]);
}

function discardUnusedSessionKeychain(keychain, session) {
  // Removing the file without `security delete-keychain` avoids the macOS 26
  // codesign poison: deleting a keychain that just signed with this identity
  // makes the next import of the same certificate fail with
  // errSecInternalComponent / "unable to build chain to self-signed root".
  restoreUserKeychains(session);
  fs.rmSync(keychain, { force: true });
}

function prepareTemporarySigningIdentity() {
  const config = loadSigningConfig();
  const { password: p12Password } = parseSigningCredentials(
    config.credentialsFile,
  );
  fs.mkdirSync(SIGNING_CACHE, { recursive: true, mode: 0o700 });
  fs.chmodSync(SIGNING_CACHE, 0o700);
  const keychain = SESSION_KEYCHAIN;
  const keychainPassword = sessionKeychainPassword();
  const session = captureUserKeychains();
  const cleanup = () => {
    restoreUserKeychains(session);
  };

  try {
    if (fs.existsSync(keychain)) {
      if (unlockSessionKeychain(keychain, keychainPassword)) {
        fs.chmodSync(keychain, 0o600);
        activateSessionKeychain(keychain, session);
        try {
          const identity = parseDeveloperIdIdentity(
            run("/usr/bin/security", [
              "find-identity",
              "-v",
              "-p",
              "codesigning",
            ]).stdout,
          );
          return { ...identity, keychain, cleanup };
        } catch {
          discardUnusedSessionKeychain(keychain, session);
        }
      } else {
        fs.rmSync(keychain, { force: true });
      }
    }

    run("/usr/bin/security", [
      "create-keychain",
      "-p",
      keychainPassword,
      keychain,
    ]);
    fs.chmodSync(keychain, 0o600);
    if (!unlockSessionKeychain(keychain, keychainPassword)) {
      fail("cached signing keychain could not be unlocked after create");
    }
    importDeveloperIdMaterial(
      keychain,
      keychainPassword,
      config,
      p12Password,
    );
    activateSessionKeychain(keychain, session);
    const identity = parseDeveloperIdIdentity(
      run("/usr/bin/security", ["find-identity", "-v", "-p", "codesigning"])
        .stdout,
    );
    return { ...identity, keychain, cleanup };
  } catch (error) {
    cleanup();
    throw error;
  }
}

function withSigningIdentity(callback) {
  const identity = prepareTemporarySigningIdentity();
  try {
    return callback(identity);
  } finally {
    identity.cleanup();
  }
}

function artifactManifest() {
  const manifestPath = process.env.FYAGENT_PRIVILEGED_MANIFEST;
  if (!manifestPath) fail("FYAGENT_PRIVILEGED_MANIFEST is missing");
  const { absolute, value } = readJson(
    manifestPath,
    "privileged artifact manifest",
  );
  if (
    value?.schema !== ARTIFACT_SCHEMA ||
    value?.variant !== "development" ||
    value?.appIdentifier !== DEV_APP_IDENTIFIER ||
    value?.helperIdentifier !== DEV_HELPER_IDENTIFIER ||
    value?.machService !== DEV_HELPER_IDENTIFIER ||
    value?.teamIdentifier !== TEAM_ID ||
    value?.helperFile !== DEV_HELPER_IDENTIFIER ||
    value?.clientFile !== CLIENT_FILE ||
    typeof value?.helperVersion !== "string" ||
    !/^\d+(?:\.\d+){0,2}$/.test(value.helperVersion)
  ) {
    fail("privileged artifact manifest is invalid or identity-mismatched");
  }
  const root = regularDirectory(
    path.dirname(absolute),
    "privileged artifact directory",
  );
  return {
    ...value,
    helperPath: regularFile(
      path.join(root, DEV_HELPER_IDENTIFIER),
      "development helper",
    ),
    clientPath: regularFile(path.join(root, CLIENT_FILE), "development client"),
  };
}

function validateRunnerExecutable(target, executable) {
  if (target !== process.env.FYAGENT_SIGNED_DEV_TARGET) {
    fail("Cargo runner target drifted");
  }
  if (!/^(?:aarch64|x86_64)-apple-darwin$/.test(target)) {
    fail("Cargo runner target is not a supported native macOS target");
  }
  const targetRoot = regularDirectory(
    path.join(ROOT, "src-tauri", "target", target),
    "Cargo target root",
  );
  const realRoot = fs.realpathSync(targetRoot);
  const binary = regularFile(executable, "Cargo application executable");
  const realBinary = fs.realpathSync(binary);
  const relative = path.relative(realRoot, realBinary);
  if (
    !relative ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  ) {
    fail("Cargo application executable escaped the current target directory");
  }
  const archs = run("/usr/bin/lipo", ["-archs", realBinary]).stdout.split(
    /\s+/,
  );
  const expected = target.startsWith("aarch64-") ? "arm64" : "x86_64";
  if (archs.length !== 1 || archs[0] !== expected) {
    fail(`Cargo application executable architecture is not ${expected}`);
  }
  return realBinary;
}

function writeInfoPlist(file, manifest, executableName, version) {
  const requirement =
    `anchor apple generic and identifier "${manifest.helperIdentifier}" ` +
    `and info[CFBundleVersion] >= "${manifest.helperVersion}" ` +
    `and certificate leaf[subject.OU] = "${TEAM_ID}"`;
  const content = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleDisplayName</key><string>FyAgent Dev</string>
  <key>CFBundleExecutable</key><string>${xml(executableName)}</string>
  <key>CFBundleIconFile</key><string>icon.icns</string>
  <key>CFBundleIdentifier</key><string>${DEV_APP_IDENTIFIER}</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>FyAgent Dev</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${xml(version)}</string>
  <key>CFBundleVersion</key><string>${xml(version)}</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>CFBundleURLTypes</key>
  <array>
    <dict>
      <key>CFBundleURLName</key><string>FyAgent Deep Link</string>
      <key>CFBundleURLSchemes</key><array><string>fyagent</string></array>
    </dict>
  </array>
  <key>SMPrivilegedExecutables</key>
  <dict>
    <key>${xml(manifest.helperIdentifier)}</key>
    <string>${xml(requirement)}</string>
  </dict>
</dict>
</plist>
`;
  fs.writeFileSync(file, content, { mode: 0o644 });
}

function sign(pathname, identity, identifier, entitlements) {
  const args = [
    "--force",
    "--sign",
    identity.hash,
    "--keychain",
    identity.keychain,
    "--identifier",
    identifier,
    "--options",
    "runtime",
    "--timestamp=none",
  ];
  if (entitlements) args.push("--entitlements", entitlements);
  args.push(pathname);
  const result = spawnSync("/usr/bin/codesign", args, {
    encoding: "utf8",
    windowsHide: true,
  });
  const stderr = `${result.stderr ?? ""}${result.stdout ?? ""}`.trim();
  if (result.error) throw result.error;
  if (result.status !== 0) {
    fail(
      `codesign exited with ${result.status}${stderr ? `: ${stderr}` : ""}`,
    );
  }
}

function verifySignature(pathname, identifier) {
  run("/usr/bin/codesign", ["--verify", "--strict", "--verbose=4", pathname]);
  const details = run("/usr/bin/codesign", ["-d", "--verbose=4", pathname]);
  const combined = `${details.stdout}\n${details.stderr}`;
  if (!combined.includes(`Identifier=${identifier}`)) {
    fail(`${identifier} signature identifier drifted`);
  }
  if (!combined.includes(`TeamIdentifier=${TEAM_ID}`)) {
    fail(`${identifier} signature team drifted`);
  }
  if (!combined.includes(`Authority=${DEVELOPER_ID_AUTHORITY}`)) {
    fail(`${identifier} is not signed by the expected Developer ID identity`);
  }
  if (/^Signature=adhoc$/m.test(combined)) {
    fail(`${identifier} is still ad-hoc signed`);
  }
  if (!/^CodeDirectory .*flags=.*runtime/m.test(combined)) {
    fail(`${identifier} is not marked for the hardened runtime`);
  }
}

function verifyLinkage(main) {
  const linked = run("/usr/bin/otool", ["-L", main]).stdout;
  if (!linked.includes(`@rpath/${CLIENT_FILE}`)) {
    fail("FyAgent executable is not linked to the privileged client dylib");
  }
  const commands = run("/usr/bin/otool", ["-l", main]).stdout;
  if (
    !/cmd LC_RPATH[\s\S]*?path @executable_path\/\.\.\/Frameworks\b/m.test(
      commands,
    )
  ) {
    fail("FyAgent executable is missing the privileged Frameworks LC_RPATH");
  }
}

function replaceAppAtomically(temporaryApp, finalApp) {
  const backup = `${finalApp}.previous-${process.pid}-${crypto.randomUUID()}`;
  const hadPrevious = fs.existsSync(finalApp);
  if (hadPrevious) fs.renameSync(finalApp, backup);
  try {
    fs.renameSync(temporaryApp, finalApp);
  } catch (error) {
    if (hadPrevious && !fs.existsSync(finalApp) && fs.existsSync(backup)) {
      fs.renameSync(backup, finalApp);
    }
    throw error;
  }
  if (hadPrevious) {
    try {
      fs.rmSync(backup, { recursive: true, force: true });
    } catch (error) {
      process.stderr.write(
        `macos-signed-dev: signed app replacement succeeded, but the previous cache bundle could not be removed: ${error instanceof Error ? error.message : String(error)}\n`,
      );
    }
  }
}

function buildSignedApp(target, executable) {
  assertFullXcode();
  const binary = validateRunnerExecutable(target, executable);
  const manifest = artifactManifest();
  const version = readCargoVersion();
  const executableName = "fyagent";
  const cacheRoot = path.join(
    os.homedir(),
    "Library",
    "Caches",
    "FyAgent",
    "SignedDev",
    target,
  );
  fs.mkdirSync(cacheRoot, { recursive: true, mode: 0o700 });
  fs.chmodSync(cacheRoot, 0o700);
  regularDirectory(cacheRoot, "signed development cache root");
  const finalApp = path.join(cacheRoot, "FyAgent Dev.app");
  const temporaryApp = path.join(
    cacheRoot,
    `.FyAgent-Dev-${process.pid}-${crypto.randomUUID()}.app`,
  );
  const contents = path.join(temporaryApp, "Contents");
  const macos = path.join(contents, "MacOS");
  const frameworks = path.join(contents, "Frameworks");
  const launchServices = path.join(contents, "Library", "LaunchServices");
  const resources = path.join(contents, "Resources");
  for (const directory of [macos, frameworks, launchServices, resources]) {
    fs.mkdirSync(directory, { recursive: true, mode: 0o755 });
  }

  const main = path.join(macos, executableName);
  const client = path.join(frameworks, CLIENT_FILE);
  const helper = path.join(launchServices, DEV_HELPER_IDENTIFIER);
  fs.copyFileSync(binary, main);
  fs.copyFileSync(manifest.clientPath, client);
  fs.copyFileSync(manifest.helperPath, helper);
  fs.copyFileSync(
    path.join(ROOT, "src-tauri", "icons", "icon.icns"),
    path.join(resources, "icon.icns"),
  );
  for (const file of [main, client, helper]) fs.chmodSync(file, 0o755);
  writeInfoPlist(
    path.join(contents, "Info.plist"),
    manifest,
    executableName,
    version,
  );

  try {
    withSigningIdentity((identity) => {
      sign(client, identity, DEV_CLIENT_IDENTIFIER);
      sign(helper, identity, DEV_HELPER_IDENTIFIER);
      sign(
        temporaryApp,
        identity,
        DEV_APP_IDENTIFIER,
        path.join(ROOT, "src-tauri", "entitlements.macos.plist"),
      );
      verifySignature(client, DEV_CLIENT_IDENTIFIER);
      verifySignature(helper, DEV_HELPER_IDENTIFIER);
      verifySignature(temporaryApp, DEV_APP_IDENTIFIER);
    });
    run("/usr/bin/codesign", [
      "--verify",
      "--deep",
      "--strict",
      "--verbose=4",
      temporaryApp,
    ]);
    verifyLinkage(main);
    replaceAppAtomically(temporaryApp, finalApp);
  } catch (error) {
    fs.rmSync(temporaryApp, { recursive: true, force: true });
    throw error;
  }
  return path.join(finalApp, "Contents", "MacOS", executableName);
}

function smokeSignIdentity(identity) {
  const smokeDir = fs.mkdtempSync(path.join(SIGNING_CACHE, "smoke-"));
  const target = path.join(smokeDir, "fyagent-codesign-smoke");
  try {
    fs.copyFileSync("/usr/bin/true", target);
    fs.chmodSync(target, 0o755);
    sign(target, identity, "com.fyagent.desktop.dev.codesign-smoke");
  } finally {
    fs.rmSync(smokeDir, { recursive: true, force: true });
  }
}

function machinePreflight() {
  assertFullXcode();
  withSigningIdentity((identity) => {
    smokeSignIdentity(identity);
    process.stdout.write(
      `Signed macOS development identity ready: ${identity.authority} [${identity.hash}]\n`,
    );
  });
}

function preflight() {
  machinePreflight();
  artifactManifest();
  process.stdout.write("Signed macOS privileged artifacts are ready\n");
}

function verifyArtifacts() {
  artifactManifest();
  process.stdout.write("Signed macOS privileged artifacts are ready\n");
}

function configure(args) {
  let p12;
  let credentials;
  for (let index = 0; index < args.length; index += 2) {
    const flag = args[index];
    const value = args[index + 1];
    if (!value) fail(`missing value for ${flag ?? "configure argument"}`);
    if (flag === "--p12") p12 = value;
    else if (flag === "--credentials") credentials = value;
    else fail(`unsupported configure argument: ${flag ?? ""}`);
  }
  if (!p12 || !credentials) {
    fail("configure requires --p12 and --credentials");
  }
  const developerIdP12 = regularFile(p12, "Developer ID PKCS#12");
  const credentialsFile = regularFile(credentials, "signing credentials file");
  parseSigningCredentials(credentialsFile);
  fs.mkdirSync(path.dirname(SIGNING_CONFIG_PATH), {
    recursive: true,
    mode: 0o700,
  });
  fs.chmodSync(path.dirname(SIGNING_CONFIG_PATH), 0o700);
  fs.writeFileSync(
    SIGNING_CONFIG_PATH,
    `${JSON.stringify(
      {
        schema: SIGNING_CONFIG_SCHEMA,
        teamIdentifier: TEAM_ID,
        developerIdP12,
        credentialsFile,
      },
      null,
      2,
    )}\n`,
    { mode: 0o600 },
  );
  fs.chmodSync(SIGNING_CONFIG_PATH, 0o600);
  process.stdout.write(
    `Configured signed macOS development at ${SIGNING_CONFIG_PATH}\n`,
  );
}

function main() {
  const [command, ...args] = process.argv.slice(2);
  if (command === "configure") {
    configure(args);
    return;
  }
  if (command === "machine-preflight" && args.length === 0) {
    machinePreflight();
    return;
  }
  if (command === "preflight" && args.length === 0) {
    preflight();
    return;
  }
  if (command === "verify-artifacts" && args.length === 0) {
    verifyArtifacts();
    return;
  }
  if (command === "app-runner") {
    const [target, executable, ...applicationArguments] = args;
    if (!target || !executable) {
      fail("app-runner requires target and executable");
    }
    if (applicationArguments.length !== 0) {
      fail(
        "signed development does not accept forwarded application arguments",
      );
    }
    const signedExecutable = buildSignedApp(target, executable);
    process.execve(signedExecutable, [signedExecutable], {
      ...process.env,
      FYAGENT_SIGNED_DEV: "1",
    });
  }
  fail(
    "expected configure, machine-preflight, preflight, verify-artifacts, or app-runner",
  );
}

try {
  main();
} catch (error) {
  process.stderr.write(
    `${error instanceof Error ? error.message : String(error)}\n`,
  );
  process.exit(1);
}
