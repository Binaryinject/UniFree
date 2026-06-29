# UniFree 补丁失败排查

## 问题
点击 "补丁 Hub" 后 Unity Hub 闪退

## 检查结果
- app.asar 没有被修改（日期仍是 Jun 17）
- 没有创建 app.asar.bak
- 授权验证仍然失败

## 可能的原因

### 1. 权限问题
需要管理员权限修改 `C:\Program Files\Unity Hub\` 目录

**解决方法：**
- 右键 unifree.exe → 以管理员身份运行
- 或者修改文件到有写权限的目录

### 2. PEM 证书未找到
如果 app.asar 中没有 PEM 证书，`find_pem_block()` 会返回 None

### 3. 代码错误导致 panic

## 下一步调试
