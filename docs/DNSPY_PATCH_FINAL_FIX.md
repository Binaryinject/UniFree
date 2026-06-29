# dnSpy 修补方法 - 最终修正

## ⚠️ 问题诊断

当前修改后仍然失败，因为：

**Index 119** 调用 `CheckSignature()` 后，会在栈上留下一个 `bool` 返回值。
**Index 120** 直接 `br.s` 跳转，但栈上还有这个未处理的值。

这会导致栈不平衡，签名验证仍然在其他地方被检查。

## ✅ 正确的修复方法

需要在跳转前先 `pop` 掉返回值：

### 需要修改 4 个指令：

#### 位置 1（Index 119-120）：

**原始：**
```il
119: callvirt   CheckSignature(...)  // 返回 bool
120: br.s       label:124            // 无条件跳转
```

**修改为：**
```il
119: callvirt   CheckSignature(...)  // 返回 bool
120: pop                             // 弹出返回值
121: br.s       label:124            // 无条件跳转
```

#### 位置 2（Index 137-138）：

**原始：**
```il
137: callvirt   CheckSignature(...)  // 返回 bool
138: br.s       label:142            // 无条件跳转
```

**修改为：**
```il
137: callvirt   CheckSignature(...)  // 返回 bool
138: pop                             // 弹出返回值
139: br.s       label:142            // 无条件跳转
```

## 📋 操作步骤

### 1. 在 dnSpy 中打开 `ValidateSignature` 方法

### 2. 右键 → `Edit IL Instructions`

### 3. 修改 Index 120：

1. **双击 Index 120**
2. **OpCode 改为：** `pop`
3. **Operand 留空**
4. 点击 OK

### 4. 在 Index 120 后插入新指令：

1. **点击 Index 120**
2. **点击 "Insert After" 按钮**
3. **OpCode 选择：** `br.s`
4. **Operand 输入：** `label:124`
5. 点击 OK

### 5. 修改 Index 138（重复步骤 3-4）：

1. **双击 Index 138**
2. **OpCode 改为：** `pop`
3. **Operand 留空**
4. 点击 OK

5. **点击 Index 138**
6. **点击 "Insert After" 按钮**
7. **OpCode 选择：** `br.s`
8. **Operand 输入：** `label:142`
9. 点击 OK

### 6. 删除多余的 throw 指令（可选）

现在 Index 121-123 和 139-141 的 throw 指令永远不会执行，可以删除它们来让代码更干净（但不是必须的）。

### 7. 保存

**File → Save Module**

## 🔍 修改后的 IL 代码

### 位置 1：
```il
116: ldloc.1
117: ldloc.3
118: ldc.i4.1
119: callvirt   CheckSignature(...)
120: pop                          // 新增：弹出返回值
121: br.s       label:124         // 修改：无条件跳转
122: ldstr      "invalid..."      // 永远不执行
123: newobj     ...               // 永远不执行
124: throw                        // 永远不执行
125: ret
```

### 位置 2：
```il
134: ldloc.1
135: ldarg.1
136: ldc.i4.1
137: callvirt   CheckSignature(...)
138: pop                          // 新增：弹出返回值
139: br.s       label:142         // 修改：无条件跳转
140: ldstr      "invalid..."      // 永远不执行
141: newobj     ...               // 永远不执行
142: throw                        // 永远不执行
143: ret
```

## 📝 总结

关键点：
1. **调用 `CheckSignature()` 会在栈上留下 `bool` 返回值**
2. **必须先 `pop` 清理栈**
3. **然后 `br.s` 无条件跳转**
4. **这样签名验证结果被完全忽略**

修改完成后，文件大小应该增加约 2-4 字节（因为插入了 2 条 `pop` 指令）。
