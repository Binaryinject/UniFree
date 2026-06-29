# dnSpy 修补方法 - 正确版本

## ⚠️ 重要更新

之前的方法（删除所有指令只留 `ret`）会破坏 DLL 的元数据，导致：
```
System.BadImageFormatException: No string associated with token
```

## ✅ 正确的修补方法

### 方法 1：修改验证方法返回值（推荐）

不要删除所有指令，而是**只修改验证结果**：

1. **在 dnSpy 中定位 `ValidateSignature` 方法**
2. **右键 → Edit IL Instructions**
3. **找到两处 `CheckSignature` 调用后的验证：**

   **位置 1（Index 120-123）：**
   ```il
   IL_0155: callvirt   System.Boolean System.Security.Cryptography.Xml.SignedXml::CheckSignature(...)
   IL_015A: brtrue.s   IL_016A      // 如果验证通过，跳转到 IL_016A (ret)
   IL_015C: ldstr      "The digital signature is invalid."
   IL_0161: newobj     System.IO.InvalidDataException::.ctor(System.String)
   IL_0166: throw
   ```

   **修改为：**
   ```il
   IL_0155: callvirt   System.Boolean System.Security.Cryptography.Xml.SignedXml::CheckSignature(...)
   IL_015A: pop        // 弹出返回值（不使用）
   IL_015B: br.s       IL_016A      // 无条件跳转到返回
   IL_015D: nop        // 填充空间
   IL_015E: nop
   IL_015F: nop
   IL_0160: nop
   IL_0161: nop
   IL_0162: nop
   IL_0163: nop
   IL_0164: nop
   IL_0165: nop
   IL_0166: nop
   ```

   **位置 2（Index 138-141）：** 同样的修改

### 方法 2：直接修改跳转（更简单）

**只修改 2 处 `brtrue.s` 指令为 `br.s`（无条件跳转）：**

1. **Index 120：**
   - 原始：`brtrue.s label:124` （如果 true 则跳转）
   - 修改为：`br.s label:124` （无条件跳转）

2. **Index 138：**
   - 原始：`brtrue.s label:142` （如果 true 则跳转）
   - 修改为：`br.s label:142` （无条件跳转）

### 操作步骤（方法 2 - 最简单）

1. **在 dnSpy 中打开 `ValidateSignature` 方法**
2. **右键 → Edit IL Instructions**
3. **滚动到 Index 120：**
   - 双击这一行
   - OpCode 下拉菜单改为：`br.s`
   - Operand 保持不变：`label:124`
   - 点击 OK

4. **滚动到 Index 138：**
   - 双击这一行
   - OpCode 下拉菜单改为：`br.s`
   - Operand 保持不变：`label:142`
   - 点击 OK

5. **点击 OK 关闭 IL 编辑器**
6. **File → Save Module**

## 🔍 原理

### 原始逻辑：
```
调用 CheckSignature() → 返回 bool
如果 true：跳转到 ret（返回）
如果 false：抛出异常 "The digital signature is invalid."
```

### 修改后：
```
调用 CheckSignature() → 返回 bool（但被忽略）
无条件跳转到 ret（返回）
永远不会抛出异常
```

## ✅ 优点

- ✅ **保留所有原始代码**（只改 2 个字节）
- ✅ **不破坏元数据**
- ✅ **DLL 大小几乎不变**
- ✅ **所有嵌入资源仍可访问**
- ✅ **最小化修改，最安全**

## 📊 修改前后对比

### 修改前（Index 120）：
```il
116: ldloc.1
117: ldloc.3
118: ldc.i4.1
119: callvirt   System.Boolean System.Security.Cryptography.Xml.SignedXml::CheckSignature(...)
120: brtrue.s   label:124    // ← 如果 true 跳转
121: ldstr      "The digital signature is invalid."
122: newobj     ...
123: throw
124: ret
```

### 修改后（Index 120）：
```il
116: ldloc.1
117: ldloc.3
118: ldc.i4.1
119: callvirt   System.Boolean System.Security.Cryptography.Xml.SignedXml::CheckSignature(...)
120: br.s       label:124    // ← 无条件跳转（忽略返回值）
121: ldstr      "The digital signature is invalid."  // 永远不会执行
122: newobj     ...  // 永远不会执行
123: throw       // 永远不会执行
124: ret
```

## 🎯 总结

只需要修改 **2 个 OpCode**：
- Index 120: `brtrue.s` → `br.s`
- Index 138: `brtrue.s` → `br.s`

这样既绕过了签名验证，又不会破坏 DLL 的完整性。
