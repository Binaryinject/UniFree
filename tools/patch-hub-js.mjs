#!/usr/bin/env node
// Patch / inspect the Unity Hub app.asar JavaScript without a Rust build.
//
// Mirrors `src-tauri/src/patcher.rs::patch_license_js` + `rewrite_hub_asar` and
// `config_patcher::update_hub_config`, so the JS side can be re-applied or
// verified on a machine that only has Node. It never touches the licensing
// client DLL or the exe fuse — use the UniFree app for those two.
//
//   node tools/patch-hub-js.mjs                        # patch (Hub must be closed)
//   node tools/patch-hub-js.mjs --check                # report patch state only
//   node tools/patch-hub-js.mjs --hub "D:\\Hub\\Unity Hub"
//   node tools/patch-hub-js.mjs --keep-signin          # 不写 DisableSignInRequired
//   node tools/patch-hub-js.mjs --keep-update          # 不写 DisableAutoUpdate
//   node tools/patch-hub-js.mjs --restore              # 用 app.asar.bak 还原 JS
//
// Patch points:
//   licenseQueryService-*.js : getLicense()             -> fake Unity Pro ULF
//                              isLicenseValid()         -> true
//                              checkEditorEntitlement() -> "ENTITLED"   (Hub 3.22+)
//                              checkUlfLicense()        -> "ENTITLED"   (Hub 3.22+)
//   licenseService-*.js      : isLicenseValid()         -> true
//   任意含 LocalConfig 默认值的 JS（DefaultLocalConfig-*.js 以及 3.22 起新增的
//   installerContainerSetup-*.js 副本）：
//                              DisableSignInRequired   -> true
//                              DisableAutoUpdate       -> true
//   %APPDATA%\UnityHub\hubConfig.json : hubDisableSignInRequired / hubDisableAutoUpdate -> true
import fs from "node:fs";
import path from "node:path";
import os from "node:os";

const FAKE_LICENSE =
  "[{startDate:'2020-01-01T00:00:00Z',stopDate:'2099-12-31T23:59:59Z',updateDate:'2020-01-01T00:00:00Z',valid:true,activated:true,canExpire:false,error:false,returned:false,maintenance:false,initialized:true,mustReactivate:false,beta:false,isLicenseActionInProgress:false,label:'Unity Pro',flow:'Unity Pro',entitlementGroupId:'F4-I0SA-6VDD-WI39-41QP-XXXX',isEGL:false,transactionId:'',licenseType:'ULF',subLabel:'',entitlementGroupType:'organization'}]";

const SIGNIN_KEY = "DisableSignInRequired]: false,";
const SIGNIN_PATCHED = "DisableSignInRequired]: true,";
const UPDATE_KEY = "DisableAutoUpdate]: false,";
const UPDATE_PATCHED = "DisableAutoUpdate]: true,";

function readAsar(file) {
  const data = fs.readFileSync(file); // 整文件读入并关闭句柄，Windows 上才能后续原子 rename
  const jsonLen = data.readUInt32LE(12);
  return {
    header: JSON.parse(data.subarray(16, 16 + jsonLen).toString("utf8")),
    baseOffset: 8 + data.readUInt32LE(4),
    data,
  };
}

function walk(node, prefix = "", out = []) {
  for (const [name, child] of Object.entries(node?.files ?? {})) {
    const p = prefix ? `${prefix}/${name}` : name;
    if (child.files) walk(child, p, out);
    else out.push({ path: p, size: child.size || 0, offset: child.offset || 0, unpacked: !!child.unpacked, executable: !!child.executable });
  }
  return out;
}

function readEntry(asar, entry) {
  const start = asar.baseOffset + Number(entry.offset);
  return asar.data.subarray(start, start + entry.size);
}

function replaceMethodBody(content, signature, body) {
  const idx = content.indexOf(signature);
  if (idx < 0) return null;
  const after = content.slice(idx);
  const braceStart = after.indexOf("{");
  if (braceStart < 0) return null;
  let depth = 0;
  let end = 0;
  for (let i = braceStart; i < after.length; i++) {
    if (after[i] === "{") depth++;
    else if (after[i] === "}") {
      depth--;
      if (depth === 0) {
        end = i + 1;
        break;
      }
    }
  }
  if (!end) return null;
  return content.replace(after.slice(0, end), body);
}

const hits = { license: [], signin: [], update: [] };

function patchJs(text, fileName, opts) {
  let t = text;
  if (fileName.startsWith("licenseQueryService")) {
    const before = t;
    t = replaceMethodBody(t, "async getLicense()", `async getLicense() {\n\t\t\t\treturn ${FAKE_LICENSE};\n\t\t\t}`) ?? t;
    t = replaceMethodBody(t, "async isLicenseValid()", "async isLicenseValid() {\n\t\t\t\treturn true; // patched by unifree\n\t\t\t}") ?? t;
    t = replaceMethodBody(t, "async checkEditorEntitlement()", 'async checkEditorEntitlement() {\n\t\t\t\treturn "ENTITLED"; // patched by unifree\n\t\t\t}') ?? t;
    t = replaceMethodBody(t, "async checkUlfLicense()", 'async checkUlfLicense() {\n\t\t\t\treturn "ENTITLED"; // patched by unifree\n\t\t\t}') ?? t;
    if (t !== before) hits.license.push(fileName);
  }
  if (fileName.startsWith("licenseService") && t.includes("isLicenseValid")) {
    const legacy = "isLicenseValid() {\n\t\tif (await this.#licenseQueryService.isLicenseValid()) return true;";
    const before = t;
    if (t.includes(legacy)) t = t.replace(legacy, "isLicenseValid() {\n\t\treturn true; // patched by unifree");
    else t = replaceMethodBody(t, "async isLicenseValid()", "async isLicenseValid() {\n\t\t\t\treturn true; // patched by unifree\n\t\t\t}") ?? t;
    if (t !== before) hits.license.push(fileName);
  }
  // 不按文件名过滤：3.22 起 installerContainerSetup-*.js 里还有一份 DefaultLocalConfig 副本
  if (opts.disableSignin && t.includes(SIGNIN_KEY)) {
    t = t.replaceAll(SIGNIN_KEY, SIGNIN_PATCHED);
    hits.signin.push(fileName);
  }
  if (opts.disableUpdate && t.includes(UPDATE_KEY)) {
    t = t.replaceAll(UPDATE_KEY, UPDATE_PATCHED);
    hits.update.push(fileName);
  }
  return t;
}

function defaultHubDir() {
  if (process.env.UNIFREE_HUB_DIR) return process.env.UNIFREE_HUB_DIR;
  const hubInfo = path.join(os.homedir(), "AppData", "Roaming", "UnityHub", "hubInfo.json");
  try {
    const p = JSON.parse(fs.readFileSync(hubInfo, "utf8")).executablePath;
    if (p) return path.dirname(p);
  } catch {}
  if (process.platform === "win32") {
    for (const base of [process.env.ProgramFiles, process.env["ProgramFiles(x86)"]]) {
      if (!base) continue;
      const p = path.join(base, "Unity Hub");
      if (fs.existsSync(path.join(p, "resources", "app.asar"))) return p;
    }
  }
  return null;
}

function hubConfigPath() {
  if (process.env.UNIFREE_HUB_APPDATA) return process.env.UNIFREE_HUB_APPDATA;
  if (process.platform === "win32") return path.join(process.env.APPDATA ?? path.join(os.homedir(), "AppData", "Roaming"), "UnityHub");
  if (process.platform === "darwin") return path.join(os.homedir(), "Library", "Application Support", "UnityHub");
  return path.join(process.env.XDG_CONFIG_HOME ?? path.join(os.homedir(), ".config"), "UnityHub");
}

function patchHubConfigJson(opts) {
  const file = path.join(hubConfigPath(), "hubConfig.json");
  if (!fs.existsSync(file)) return `hubConfig.json 不存在（${file}），跳过`;
  try {
    const config = JSON.parse(fs.readFileSync(file, "utf8"));
    const changed = [];
    if (opts.disableSignin && config.hubDisableSignInRequired !== true) {
      config.hubDisableSignInRequired = true;
      changed.push("hubDisableSignInRequired");
    }
    if (opts.disableUpdate && config.hubDisableAutoUpdate !== true) {
      config.hubDisableAutoUpdate = true;
      changed.push("hubDisableAutoUpdate");
    }
    if (changed.length === 0) return "hubConfig.json 已是目标状态";
    fs.writeFileSync(file, JSON.stringify(config));
    return `hubConfig.json 已更新：${changed.join(", ")}`;
  } catch (error) {
    return `hubConfig.json 更新失败：${error.message}`;
  }
}

const argv = process.argv.slice(2);
const opts = {
  disableSignin: !argv.includes("--keep-signin"),
  disableUpdate: !argv.includes("--keep-update"),
};
const checkOnly = argv.includes("--check");
const restore = argv.includes("--restore");
const hubIdx = argv.indexOf("--hub");
const hubDir = hubIdx >= 0 ? argv[hubIdx + 1] : defaultHubDir();
if (!hubDir) {
  console.error("找不到 Unity Hub 目录，请用 --hub \"<path>\" 指定");
  process.exit(2);
}

const asarPath = path.join(hubDir, "resources", "app.asar");
const bakPath = `${asarPath}.bak`;
if (!fs.existsSync(asarPath)) {
  console.error(`找不到 ${asarPath}`);
  process.exit(2);
}

// ---- 状态检查 ----
function inspect(file) {
  const asar = readAsar(file);
  const files = walk(asar.header);
  const state = { verdict: false, license: false, signin: 0, update: 0, entries: files.length, bakPatched: null };
  for (const e of files) {
    if (!e.path.endsWith(".js")) continue;
    const text = readEntry(asar, e).toString("utf8");
    if (text.includes('checkEditorEntitlement() {\n\t\t\t\treturn "ENTITLED"')) state.verdict = true;
    if (text.includes("return true; // patched by unifree")) state.license = true;
    state.signin += text.split(SIGNIN_PATCHED).length - 1;
    state.update += text.split(UPDATE_PATCHED).length - 1;
  }
  return state;
}

const current = inspect(asarPath);
console.log(`asar: ${asarPath} (${current.entries} 条目)`);
console.log(`  许可 JS 补丁 (getLicense/isLicenseValid) : ${current.license ? "已打" : "未打"}`);
console.log(`  三态 verdict 补丁 (checkEditorEntitlement): ${current.verdict ? "已打" : "未打"}`);
console.log(`  DisableSignInRequired = true 命中         : ${current.signin}`);
console.log(`  DisableAutoUpdate = true 命中             : ${current.update}`);

if (checkOnly) process.exit(0);

if (restore) {
  if (!fs.existsSync(bakPath)) {
    console.error("没有 app.asar.bak，无法还原");
    process.exit(1);
  }
  fs.renameSync(asarPath, `${asarPath}.patched`);
  fs.renameSync(bakPath, asarPath);
  console.log(`已还原 ${asarPath}（补丁版留作 app.asar.patched）`);
  process.exit(0);
}

if (!fs.existsSync(bakPath)) {
  fs.copyFileSync(asarPath, bakPath);
  console.log(`已创建备份 ${bakPath}`);
} else {
  const bakState = inspect(bakPath);
  if (bakState.license || bakState.verdict) {
    console.log("⚠ 现有 app.asar.bak 本身含补丁标记（不是纯净原版）；重建结果仍会补齐全部补丁点");
  }
}

// ---- 从备份重建（幂等）----
const src = readAsar(bakPath);
const entries = walk(src.header);
const chunks = [];
const headerEntries = [];
let offset = 0;
for (const e of entries) {
  if (e.unpacked) {
    headerEntries.push([e.path, { size: e.size, unpacked: true }]);
    continue;
  }
  let content = readEntry(src, e);
  const name = path.basename(e.path);
  if (name.endsWith(".js")) {
    const before = content.toString("utf8");
    const after = patchJs(before, name, opts);
    if (after !== before) content = Buffer.from(after, "utf8");
  }
  const ent = { size: content.length, offset: String(offset) };
  if (e.executable) ent.executable = true;
  headerEntries.push([e.path, ent]);
  chunks.push(content);
  offset += content.length;
}

const root = { files: {} };
const insert = (map, comps, value) => {
  const [head, ...rest] = comps;
  if (rest.length === 0) return void (map[head] = value);
  map[head] ??= { files: {} };
  insert(map[head].files, rest, value);
};
for (const [p, ent] of headerEntries) insert(root.files, p.split("/"), ent);

const jsonBytes = Buffer.from(JSON.stringify(root), "utf8");
const aligned = jsonBytes.length + ((4 - (jsonBytes.length % 4)) % 4);
const jsonPadded = Buffer.alloc(aligned);
jsonBytes.copy(jsonPadded);
const head = Buffer.alloc(16);
head.writeUInt32LE(4, 0);
head.writeUInt32LE(aligned + 8, 4);
head.writeUInt32LE(aligned + 4, 8);
head.writeUInt32LE(jsonBytes.length, 12);

const outPath = `${asarPath}.tmp`;
fs.writeFileSync(outPath, Buffer.concat([head, jsonPadded, ...chunks]));
try {
  fs.renameSync(outPath, asarPath); // 同卷原子替换，避免半写
} catch (error) {
  fs.rmSync(outPath, { force: true });
  console.error(`写回失败（Hub 是否仍在运行？完全退出后重试）：${error.message}`);
  process.exit(1);
}

console.log(`已写回 ${asarPath}（共 ${entries.length} 个条目）`);
console.log(`  许可 JS: ${hits.license.length ? hits.license.join(", ") : "无命中"}`);
console.log(`  DisableSignInRequired: ${hits.signin.length ? hits.signin.join(", ") : "无命中"}`);
console.log(`  DisableAutoUpdate: ${hits.update.length ? hits.update.join(", ") : "无命中"}`);
console.log(`  ${patchHubConfigJson(opts)}`);
console.log("请完全退出 Hub 后重新启动；asar 完整性 fuse 需为关闭状态（由 UniFree 负责）。");
