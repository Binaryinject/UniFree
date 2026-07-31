# Unity Editor DLL 补丁方法

## 概述

Unity Editor 使用 `Unity.Licensing.EntitlementResolver.dll`（< 6000.7）或
`Unity.Licensing.Client.exe`（>= 6000.7，.NET 10 Native AOT）验证许可证签名。
补丁目标是绕过 `ValidateSignature` 方法，使其接受任何签名的 ULF 文件。

## 版本分界

| Unity 版本 | 目标文件 | 文件类型 | 补丁方法 |
|-----------|---------|---------|---------|
| < 6000 | `System.Security.Cryptography.Xml.dll` | .NET IL DLL | 替换预编译 DLL |
| 6000.0 ~ 6000.6 | `Unity.Licensing.EntitlementResolver.dll` | .NET IL DLL | 替换预编译 DLL |
| **>= 6000.7** | **`Unity.Licensing.Client.exe`** | **Native AOT PE** | **字节级 patch（见下文）** |

---

## Native AOT 补丁（Unity >= 6000.7）

### 目标文件

```
{Editor}/Data/Resources/Licensing/Client/Unity.Licensing.Client.exe
```

例如：`C:\Program Files\Unity\Hub\Editor\6000.7.0a3\Editor\Data\Resources\Licensing\Client\Unity.Licensing.Client.exe`

### 实现位置

`src-tauri/src/patcher.rs` → `patch_licensing_client()`

### 二进制结构

- PE 格式，约 22MB，~66,000 个函数
- `.text` 节：VA 基址 = `0x1000`，文件偏移 = `0x400`
- **文件偏移 = VA - 0xC00**（即 `VA - 0x1000 + 0x400`）
- `.rdata` 节：VA 范围约 `0x140941000` - `0x141249000`
- `.data` 节：VA 范围约 `0x141249000` - `0x1413cc000`

### 许可证验证流程（逆向分析结论）

```
UlfLicense.Parse()
  → XmlReader.Read()
    → [前置] sub_1404F1360 检查文档含 <Signature> 元素（无签名 ULF 在此被拒）
    → XmlExtensions.ValidateSignature 包装函数 (sub_1404F1C10)
        ↑ ★ 唯一补丁点：函数头 → mov eax,1; ret
        ↑ Parse 调用它但忽略返回值；失败路径通过异常/__debugbreak 中断。
        ↑ 短路后以下整条验证链路不再执行：
      │
      ├─[门控] if (!hasSignature || !checkEnabled) → 跳到 LABEL_14
      │
      ├─ CheckSignature()  → BCryptVerifySignature / NCryptVerifySignature
      │
      ├─ 时间检查 (NotBefore / NotAfter)
      │
      ├─ 公钥比较: sub_1401708C0 (SequenceEqual)
      │   比较 ULF XML 中的 <RSAKeyValue> 与 Unity 内嵌证书公钥
      │   ⚠️ UniFree 使用随机 RSA 密钥对，此比较永远失败！
      │
      └─[LABEL_14] sub_14042AE40(doc, cert, a3=1)
           └─ sub_14042ADC0(doc, cert) ← 早期版本的 P3 目标
```

**单点补丁原理**：Parse 流程在 `sub_140738C80` 中以硬编码 `a3=1` 调用 ValidateSignature
包装函数，但**忽略其返回值**——该函数的失败路径靠抛异常中断。把包装函数头改成
`mov eax,1; ret` 后，整个验证链路全部短路，签名 ULF 原样通过。

### 补丁一览

| Patch | 位置 (函数/指令) | 锚点模式 | 补丁内容 |
|-------|-----------------|---------|---------|
| **P1** | `ValidateSignature` 包装函数 `sub_1404F1C10` 函数头 | `57 56 53 48 83 EC 20 48 8B DA 48 85 DB 74 ?? 48 8B 71 10 40 0F B6 79 18 48 8D 15 ?? ?? ?? ?? 48 39 11 74 ?? 48 8D 15 ?? ?? ?? ?? 48 39 11 75 ?? 48 8B D3` | 函数头 6 字节 → `B8 01 00 00 00 C3`（mov eax,1; ret） |

**单个补丁即为完整方案**。历史版本为 4 补丁（P1 证书链 / P2 门控 / P3 LABEL_14 /
P4 BCrypt），先精简为 P2+P3，再演进为当前的单点包装函数补丁。详见更新日志。

### 已验证版本

| 版本 | 文件大小 | 补丁结果 |
|------|---------|---------|
| 6000.7.0a3 | 22,086,568 bytes | ✅ 单点补丁应用成功（1 个写入点，自动测试验证） |

### 排查方法

#### 日志位置

```
C:\Users\{用户名}\AppData\Local\Unity\Unity.Licensing.Client.log
```

关键日志行：
- `Exception caught while parsing license` — ULF 解析失败
- `System.IO.InvalidDataException: The digital signature is invalid.` — 签名验证失败
- `Processed 0 license files` — 0 个许可证被加载
- `Found 0 entitlement groups` — Editor 收到的结果是 0 个授权组 → 显示 "No valid license"

#### 进程管理

补丁后必须杀掉旧的 Licensing Client 进程再重开 Editor：

```powershell
taskkill /F /IM Unity.Licensing.Client.exe
```

补丁前也应确保没有 Editor 正在使用该文件。

#### 验证补丁是否生效

```bash
exe="path/to/Unity.Licensing.Client.exe"
# P1: 期望 b801010000eb139090
dd if="$exe" bs=1 skip=$((0x40D22B)) count=9 2>/dev/null | xxd -p
# P2: 期望 eb46
dd if="$exe" bs=1 skip=$((0x4F02D9)) count=2 2>/dev/null | xxd -p
# P3: 期望 b801000000c3
dd if="$exe" bs=1 skip=$((0x42A1C0)) count=6 2>/dev/null | xxd -p
```

### 适配新版本的 IDA MCP 工作流

当新版本 Unity 发布，锚点模式可能失效。以下是发现新补丁点的流程：

#### 1. 确认失败点

查看 `Unity.Licensing.Client.log` 中的异常调用栈。`ValidateSignature + 0x565`
这样的偏移可以帮助在 IDA 中定位函数。

#### 2. 定位 ValidateSignature 函数

Native AOT 不保留托管方法名作为符号（`lookup_funcs` 找不到），
需要间接定位：

```
# 搜索关键字符串
mcp__ida__find_string("digital signature")

# 搜索 BCrypt/NCrypt 导入的交叉引用
mcp__ida__xrefs_to(BCryptVerifySignature_thunk_VA)

# 从 xref 回溯到调用者函数，找到 ValidateSignature
```

#### 3. 分析验证流程

```python
# 反编译可疑函数
mcp__ida__decompile(function_VA)

# 查看调用者和被调用者
mcp__ida__callers(function_VA)
mcp__ida__callees(function_VA)
```

#### 4. 找到门控指令（P2 等价物）

在 ValidateSignature 反编译中找：
```c
if ((hasSignature & checkEnabled) == 0)
    goto LABEL_14;  // 跳过签名验证
```

对应的汇编是 `test r1, r2; jz +offset`。

#### 5. 提取锚点字节

```python
mcp__ida__get_bytes(address, size=30)
```

用 IDA 中的确切字节构建通配符模式，然后用测试程序验证唯一性：

```rust
// 临时测试程序：验证锚点在文件中只匹配一次
fn main() {
    let data = fs::read("Unity.Licensing.Client.exe").unwrap();
    let anchor = "新锚点模式";
    let mut off = 0usize;
    let mut count = 0;
    while let Some(m) = find_pattern_at(&data[off..], anchor) {
        println!("Match {}: 0x{:X}", count, off + m);
        count += 1;
        off += m + 1;
    }
    println!("Total: {} (should be 1)", count);
}
```

#### 6. 锚点设计原则

- **唯一性优先**：锚点在 22MB 文件中必须只命中一次
- **保守通配**：对绝对地址（call 偏移、栈偏移）使用 `??`，对寄存器操作码保持精确
- **测试验证**：始终在 .bak（干净副本）上测试，确保所有锚点正确命中
- **上下文包围**：尽量包含目标指令前后足够多的上下文字节
- **避免匹配开头**：`find_pattern` 返回第一个匹配，如果锚点在文件中有多个命中，取第一个会导致补丁错位

### Version-specific hardcoded offsets (for shell/dd manual patching)

以下偏移仅适用于 **6000.7.0a3** 版本的参考，patch.rs 使用锚点模式匹配，
不依赖硬编码偏移：

| Patch | 文件偏移 | 原始字节 | 补丁字节 |
|-------|---------|---------|---------|
| P1 (包装函数) | 0x4F1010 | 5756534883EC20 | B801000000C3 |

---

## 旧版 .NET IL DLL 补丁（< 6000.7）

### 补丁原理

`ValidateSignature` 方法位于 DLL 中，负责：
1. 验证许可证文件的 XML 数字签名
2. 验证 PACL（来自 Unity 服务器）的签名

#### IL 级补丁方法

1. 搜索错误字符串 `"The digital signature is invalid."` 的 UTF-16 编码
2. 向前搜索 IL 指令模式：`ldstr` (0x72) + `newobj` (0x73) + `throw` (0x2A)
3. 将这三条指令 NOP 掉（替换为 0x00）

```
原始 IL:
  ldstr "The digital signature is invalid."  // 0x72 XX XX XX XX
  newobj InvalidDataException                // 0x73 XX XX XX XX
  throw                                      // 0x2A

补丁后:
  nop                                        // 0x00
  nop                                        // 0x00
  nop                                        // 0x00
```

4. 搜索 `CheckSignature` 字符串
5. 向前搜索 `callvirt` (0x6F) + `brtrue.s` (0x2D) 模式
6. 将 `brtrue.s` NOP 掉（忽略 CheckSignature 返回值）

```
原始 IL:
  callvirt CheckSignature                    // 0x6F XX XX XX XX
  brtrue.s <label>                           // 0x2D XX

补丁后:
  callvirt CheckSignature                    // 0x6F XX XX XX XX
  nop                                        // 0x00
  nop                                        // 0x00
```

### 当前实现

对于 < 6000.7 版本，`patch_entitlement_resolver()` 直接替换为预编译的补丁 DLL。
预编译 DLL 按 Unity 主版本组织在 `src-tauri/resources/win/` 目录下。

### 版本兼容性

| DLL 版本 | 文件大小 | 补丁方法 |
|---------|---------|---------|
| 1.18.1 (Editor) | ~341KB | 预编译 DLL |
| 1.17.4 (Hub) | ~514KB | 不兼容，需 asar 补丁 |

## 相关文件

- `src-tauri/src/patcher.rs` — 补丁实现（包含 Native AOT 和 IL DLL 两种路径）
- `src-tauri/src/scanner.rs` — 版本检测（`is_native_aot_editor`）
- `src-tauri/src/ulf_signer.rs` — ULF 签名生成
- `src-tauri/resources/win/` — 预编译补丁 DLL（< 6000.7 版本）

## 更新日志

- 2026-07-31: 精简为单点补丁 —— 直接短路 ValidateSignature 包装函数 (sub_1404F1C10)。
  逆向确认：Parse 调用它但忽略返回值，失败靠异常中断；前置检查 sub_1404F1360 要求文档
  必须含 `<Signature>` 元素（无签名 ULF 不可行），因此保留签名 ULF、只打这一个函数头。
  锚点含配置字段读取 + 双重类型检查，唯一命中。注意 `cmp [rcx],rdx` 编码为 `48 39 11`
  （ModRM 0x11，rm=rcx），误写 17 会匹配失败。
- 2026-07-31: 精简为 P2+P3 两个补丁。IDA 实证确认：P1 仅经 sub_14042AE40 的 a3==0
  分支可达（本工具不走）；P4 的字节模式全文件命中 19 处（含无关函数），全量打补丁会
  误改代码。移除后补丁点从 21 个写入点降到 2 个。
- 2026-07-28: 完成 Unity 6000.7+ Native AOT 字节级补丁（4 个 patch 点），更新文档
- 2026-07-04: 添加离线二进制补丁工具（tools/patch-dll），按版本组织 DLL
- 2026-07-04: 简化为直接使用预编译 DLL，移除二进制补丁逻辑
