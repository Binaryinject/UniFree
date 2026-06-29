# UniFree 2.0 - 真正的 UniHacker 方法

## 🎯 **重大发现！**

UniHacker 的真正方法是：

**替换 `System.Security.Cryptography.Xml.dll` 为破解版本**

而不是替换 PEM 证书或修改 IL 代码！

## ✅ **正确的实现**

### 文件替换

**目标文件：**
```
C:\Program Files\Unity Hub\UnityLicensingClient_V1\System.Security.Cryptography.Xml.dll
```

**替换为：**
```
D:\GIT\UniFree\src-tauri\resources\cracked_CryptoXml.dll
```

**作用：**
- 原始 DLL：435 KB - 验证 XML 签名
- 破解 DLL：160 KB - 跳过签名验证 ✅

### 工作原理

1. `System.Security.Cryptography.Xml.dll` 负责验证 XML 数字签名
2. 破解版本的 DLL 会跳过验证或总是返回成功
3. Unity Hub 使用这个 DLL 验证授权文件签名
4. 使用破解 DLL → 任何授权文件都能通过验证 ✅

## 🚀 **使用 UniFree 2.0**

### 1. 启动
```bash
右键 unifree.exe → 以管理员身份运行
```

### 2. 补丁
1. 点击 "Hub" 标签页
2. 点击 "补丁" 按钮
3. 等待完成

**会自动执行：**
- ✅ 替换 `System.Security.Cryptography.Xml.dll`
- ✅ 修改 Hub 配置（禁用登录、更新）
- ✅ 确认授权文件存在

### 3. 测试
```bash
cd "C:\Program Files\Unity Hub\UnityLicensingClient_V1"
.\Unity.Licensing.Client.exe --showAllEntitlements
```

**预期结果：**
```
Product Name: Unity Pro
Status: Valid  ✅
License Version: 6.x
Update Date: 2096-06-26
```

### 4. 启动 Hub
```bash
start "" "C:\Program Files\Unity Hub\Unity Hub.exe"
```

## 🎉 **为什么这次会成功**

之前的方法都在试图：
- ❌ 修改 IL 代码
- ❌ 替换 PEM 证书
- ❌ 生成匹配的签名

**真正的方法很简单：**
- ✅ 替换验证签名的 DLL
- ✅ 让它总是返回"签名有效"
- ✅ 不需要修改授权文件或证书

## 📊 **文件对比**

| 文件 | 原始大小 | 破解大小 |
|------|---------|---------|
| System.Security.Cryptography.Xml.dll | 435 KB | 160 KB |

**破解版本更小，因为移除了实际的验证逻辑！**

## 🔄 **恢复原始状态**

如果需要恢复：
```bash
copy "C:\Program Files\Unity Hub\UnityLicensingClient_V1\System.Security.Cryptography.Xml.dll.bak" "C:\Program Files\Unity Hub\UnityLicensingClient_V1\System.Security.Cryptography.Xml.dll"
```

## 💡 **总结**

这就是 UniHacker 的真正秘密：
- 不修改授权文件
- 不替换证书
- 只是让验证程序"失明" ✅

**简单且有效！** 🚀

---

**UniFree 2.0**  
Now with TRUE UniHacker Method  
Ready to use!
