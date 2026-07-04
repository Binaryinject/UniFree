# Unity Editor DLL 补丁方法

## 概述

Unity Editor 使用 `Unity.Licensing.EntitlementResolver.dll` 验证许可证签名。补丁目标是绕过 `ValidateSignature` 方法，使其接受任何签名（包括假签名）。

## 补丁原理

### 目标方法

`ValidateSignature` 方法位于 `Unity.Licensing.EntitlementResolver.dll` 中，负责：
1. 验证许可证文件的 XML 数字签名
2. 验证 PACL（来自 Unity 服务器）的签名

### 验证流程

```
ValidateSignature()
├── 检查许可证签名 → CheckSignature() → 失败则抛出 InvalidDataException
└── 检查 PACL 签名 → CheckSignature() → 失败则抛出 InvalidDataException
```

### 补丁策略

**方法一：二进制补丁（推荐用于新版本）**

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

**方法二：替换预编译 DLL（简单可靠）**

直接使用已补丁的 DLL 替换原始文件。适用于：
- 二进制补丁失败（字节模式不匹配）
- DLL 版本与预编译版本匹配

## 版本兼容性

### 已知版本

| DLL 版本 | 文件大小 | 补丁方法 |
|---------|---------|---------|
| 1.18.1 (Editor) | ~341KB | 预编译 DLL |
| 1.17.4 (Hub) | ~514KB | 不兼容，需 asar 补丁 |

### 版本变化检测

1. **检查文件大小**：原始 DLL ~514KB，补丁后 ~341KB
2. **检查备份文件**：`.bak` 文件存在表示已补丁
3. **检查字符串**：搜索 "The digital signature is invalid."

## 实现代码

```rust
/// 补丁 EntitlementResolver.dll
/// 
/// # 参数
/// - `dll_path`: DLL 文件路径
/// 
/// # 返回
/// - `Ok(String)`: 补丁成功消息
/// - `Err(String)`: 错误信息
pub fn patch_entitlement_resolver(dll_path: &str) -> Result<String, String> {
    let path = Path::new(dll_path);
    if !path.exists() {
        return Err("DLL not found".into());
    }

    // 创建备份
    let bak_path = format!("{}.bak", dll_path);
    if !Path::new(&bak_path).exists() {
        fs::copy(path, &bak_path).map_err(|e| e.to_string())?;
    }

    // 使用预编译的补丁 DLL
    let patched_dll = include_bytes!("../resources/win/Unity.Licensing.EntitlementResolver.dll");
    fs::write(path, patched_dll).map_err(|e| format!("Failed to write patched DLL: {}", e))?;
    
    Ok("Patched: replaced with pre-patched DLL".into())
}
```

## 补丁后行为

1. **许可证验证**：任何带有 `<Signature>` 节点的 ULF 文件都会被接受
2. **签名内容**：签名可以是假的（如 `<Signature>dummy</Signature>`）
3. **许可证格式**：必须符合 Unity 许可证 XML 格式

## 故障排除

### 问题：二进制补丁失败

**原因**：DLL 版本更新，字节模式不匹配

**解决**：
1. 使用 dnlib 或 ILSpy 反编译新版本 DLL
2. 查找 `ValidateSignature` 方法
3. 更新字节模式或创建新的预编译 DLL

### 问题：补丁后许可证仍无效

**原因**：许可证格式错误或缺少必要字段

**检查**：
1. 许可证必须有 `<Signature>` 节点
2. 许可证必须有正确的 XML 结构
3. 机器绑定信息必须匹配

### 问题：Hub 版本不兼容

**原因**：Hub 使用不同版本的 Licensing Client

**解决**：使用 asar 补丁方法（见 hub-asar-patching.md）

## 相关文件

- `src-tauri/resources/win/Unity.Licensing.EntitlementResolver.dll` - 预编译补丁 DLL
- `src-tauri/src/patcher.rs` - 补丁逻辑实现
- `src/components/EditorTab.tsx` - 前端 UI

## 更新日志

- 2026-07-04: 简化为直接使用预编译 DLL，移除二进制补丁逻辑
