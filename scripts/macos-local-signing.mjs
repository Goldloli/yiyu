import { spawnSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const identityName = "Yiyu Local Development";

function command(program, args, options = {}) {
  const result = spawnSync(program, args, { encoding: "utf8", ...options });
  if (result.error || result.status !== 0) {
    throw new Error(`${program} 执行失败${result.stderr ? `：${result.stderr.trim()}` : ""}`);
  }
  return result.stdout?.trim() ?? "";
}

function defaultKeychain() {
  return command("security", ["default-keychain", "-d", "user"]).replace(/^"|"$/g, "");
}

function signingIdentity(keychain) {
  const output = command("security", ["find-identity", "-p", "codesigning", keychain]);
  const identities = [...output.matchAll(/^\s*\d+\)\s+([A-F0-9]{40})\s+"Yiyu Local Development"/gm)];
  if (identities.length > 1) throw new Error("找到多个同名签名身份，请先在钥匙串中处理重复项。");
  return identities[0]?.[1];
}

function setup(keychain) {
  if (signingIdentity(keychain)) {
    console.log("本机开发签名身份已存在。");
    return;
  }
  const certificate = spawnSync("security", ["find-certificate", "-c", identityName, keychain], { encoding: "utf8" });
  if (certificate.status === 0) throw new Error("已存在同名证书但没有对应私钥；请先检查钥匙串，避免更换已有签名身份。");
  if (certificate.error || certificate.status !== 44) throw new Error("无法检查钥匙串中的现有证书。");

  const originalUmask = process.umask(0o077);
  const directory = mkdtempSync(join(tmpdir(), "yiyu-signing-"));
  const config = join(directory, "openssl.cnf");
  const key = join(directory, "key.pem");
  const cert = join(directory, "cert.pem");
  const archive = join(directory, "identity.p12");
  const password = randomBytes(24).toString("hex");
  try {
    writeFileSync(config, `[req]\ndistinguished_name=dn\nx509_extensions=ext\nprompt=no\n[dn]\nCN=${identityName}\n[ext]\nbasicConstraints=critical,CA:TRUE\nkeyUsage=critical,digitalSignature,keyCertSign,cRLSign\nextendedKeyUsage=codeSigning\n`);
    command("/usr/bin/openssl", ["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-sha256", "-days", "3650", "-config", config, "-keyout", key, "-out", cert]);
    command("/usr/bin/openssl", ["pkcs12", "-export", "-inkey", key, "-in", cert, "-out", archive, "-passout", "env:YIYU_LOCAL_SIGNING_PASSWORD"], {
      env: { ...process.env, YIYU_LOCAL_SIGNING_PASSWORD: password }
    });
    command("security", ["import", archive, "-k", keychain, "-P", password, "-T", "/usr/bin/codesign"]);
    if (!signingIdentity(keychain)) throw new Error("证书已导入，但未找到对应签名身份；请检查钥匙串。");
    console.log("已创建本机开发签名身份，没有修改系统证书信任设置。");
  } finally {
    rmSync(directory, { recursive: true, force: true });
    process.umask(originalUmask);
  }
}

function build(keychain) {
  const identity = signingIdentity(keychain);
  if (!identity) throw new Error("尚未配置本机开发签名，请先运行 pnpm macos:signing:setup。");
  const env = { ...process.env, APPLE_SIGNING_IDENTITY: identity };
  for (const name of ["APPLE_CERTIFICATE", "APPLE_CERTIFICATE_PASSWORD", "APPLE_ID", "APPLE_PASSWORD", "APPLE_TEAM_ID", "APPLE_API_KEY", "APPLE_API_ISSUER", "APPLE_API_KEY_PATH"]) delete env[name];
  const result = spawnSync("pnpm", ["tauri", "build", "--debug", "--bundles", "app"], { stdio: "inherit", env });
  if (result.error || result.status !== 0) throw new Error("本机开发构建失败。");
}

try {
  if (process.platform !== "darwin") throw new Error("本机开发签名仅适用于 macOS。");
  const keychain = defaultKeychain();
  if (process.argv[2] === "setup") setup(keychain);
  else if (process.argv[2] === "build") build(keychain);
  else throw new Error("用法：node scripts/macos-local-signing.mjs setup|build");
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
