# macOS 编辑器侧：Licensing Client 代码签名校验（RE 记录）

macOS 上 Unity 编辑器在启动 Licensing Client 之后，会先验证 **cl Licensing Client 可执行文件本身的
代码签名**，通过后才做 IPC 握手。任何被修改过的 client（IL 补丁后重签、替换成别的版本的 client、
shell shim）都会在这一步被拒：

```
[Licensing::Client] Code 8 while verifying Licensing Client signature (process Id: …, path: "…")
[Licensing::Module] LicensingClient has failed validation; ignoring
[Licensing::Client] Error: HandshakeResponse reported an error:
[Licensing::Module] Error: Failed to handshake to channel: "LicenseClient-<user>"
```

这与 Windows 2019.4 的 `LicensingClient::ValidateServerProcess`（Authenticode 校验）是同一类关卡，
因此 macOS 侧也必须先打掉编辑器里的这个判定，否则客户端补丁永远不会被采纳。

## 已验证的锚点（Unity 6000.3.28f1，arm64）

| 项 | 值 |
|----|----|
| 失败日志格式串 | `Code {0} while verifying Licensing Client signature (process Id: {1}, path: "{2}")` |
| 格式串 VA / 文件偏移 | `0x104613a03` / `0x4613a03`（`__TEXT,__cstring`，长度 0x52） |
| 校验调用 | `0x102862228  bl 0x1031f71bc` → 返回码在 `w0` |
| 判定 | `0x102862230  cmp w0, #9`；`0x102862234  b.ne 0x102862240`（`0x54000061`） |
| 成功分支 | `0x102862238  mov w20, #1`（返回 1 = 校验通过） |
| 失败分支 | 打印上面那条 `Code {0} while verifying…` 日志并返回 0 |

补丁（与 Windows 2019 的 P1 同构）：

```
文件偏移 0x2862234: 61 00 00 54  (b.ne)  →  1f 20 03 d5  (nop)
```

即把「结果码 != 9 就失败」改成「恒走成功分支」。本机实测：打完后 `Code 8 while verifying…`
与 `failed validation; ignoring` 全部消失，编辑器直接进入握手阶段。

> 该偏移只对 6000.3.28f1 成立。跨版本适配应改为按**格式串 + 前后指令形态**定位：
> `adrp/add <fmt string>` 之后的 `bl <verify>; cmp w0,#9; b.ne <fail>`，再对 `b.ne` 写入 NOP。

## 客户端替换行不通（已排除）

把编辑器自带的 client 换成一个 shim，指向 **Hub 的已补丁托管 client（LocalIPC 1.18.3）**：

- 打完上面的编辑器补丁后，签名校验通过、握手确实开始了；
- 但 6000.3.28f1 的编辑器走 `LicenseClient-<user>-1.19.0` 通道，1.18.3 的 Hub client 无法完成握手
  （`HandshakeResponse reported an error`），拿不到 entitlement。

所以 **client 必须由对应版本自己提供**，AOT 版本就得在 AOT 二进制上打补丁。

## AOT client 侧（6000.3.28+）现状

`Unity.app/Contents/Helpers/UnityLicensingClient.app/Contents/MacOS/` 下只有一个 arm64 Mach-O
（Native AOT，无托管 DLL），`.text/__managedcode` + `__modules` + `.dotnet_eh_table` 结构。
运行时会因为 ULF 签名不匹配抛：

```
System.IO.InvalidDataException: The digital signature is invalid.
```

已确认的问题与线索：

- 该错误串在二进制里的 UTF-16 副本位于 `__TEXT` 0xdb107f 附近，但**全量扫描没有找到任何
  `adrp`/`add` 直接引用该页**，说明 AOT 的字符串字面量不是被代码直接寻址，而是经运行时字符串
  表/冻结段间接取得（启动期可在 0x100358060 附近的运行时 helper 里观察到对字符串区域的读）。
- 因此「按字符串 xref 找 throw 点」这条路在 AOT 上走不通，需要改用动态手段定位：
  1. lldb 在 `read`/`write` 上挂钩，抓 ULF 解析与异常日志的真实调用栈（获取 `__managedcode`
     区间内的函数地址：0x1000a87a0 起）；
  2. 或解析 Native AOT 的 `__modules` / MethodDesc → CodeManager 映射，用 `ValidateSignature`
     的方法名元数据反查原生入口；
  3. 命中后与 Windows 思路一致：把校验函数入口改成 `mov w0,#1; ret`
     （`0x52800020 0xd65f03c0`）。

托管 client（6000.3.21–6000.3.27 等）不受此限制：IL 就地补丁 + `DOTNET_ReadyToRun=0`
启动器 shim 已验证可让 ULF 通过校验。
