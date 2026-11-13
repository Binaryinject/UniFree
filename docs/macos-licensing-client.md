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

## AOT client 侧（6000.3.28+）：已拿到托管栈回溯

AOT 客户端**保留了托管方法名的栈回溯**（Native AOT 的 stack trace 支持），触发方式就是让编辑器连接它并请求
entitlement，日志里会打印完整调用链：

```
System.IO.InvalidDataException: The digital signature is invalid.
   at Unity.Licensing.EntitlementResolver.Xml.XmlExtensions.ValidateSignature(XmlDocument, X509Certificate2, Boolean, String) + 0x590
   at Unity.Licensing.EntitlementResolver.Xml.XmlReader`1.Read(Stream, Boolean, Boolean) + 0xdc
   at Unity.Licensing.EntitlementResolver.Ulf.UlfLicense.Parse(String, X509Certificate2) + 0xc0
   at Unity.Licensing.Client.Services.Helpers.LicenseFilesMapper.MapLicenseFilesToResult(IEnumerable`1, Boolean) + 0x130
```

这条链路给出了三个可直接利用的事实：

1. 目标函数就是 `XmlExtensions.ValidateSignature`，且**抛出点在方法内 +0x590**；
2. 调用点偏移：`Read + 0xdc` 调它、`Parse + 0xc0` 调 `Read`、`MapLicenseFilesToResult + 0x130` 调 `Parse`；
3. 方法名 `ValidateSignature` 在二进制里以 ASCII 出现在元数据区（文件 0xb6fb2d），可用于后续反查 MethodDesc。

### 已排除：按 call-site 直接定位

用 DYLD 拦截 `read` 抓到了 ULF 读取时的真实调用栈（需用 `__DATA,__interpose` 方式，且多数调用是
`blr` 间接调用），但栈帧返回地址前一条指令**多数不是 `bl` 而是 `blr`**，因此无法直接从 `Read+0xd8`
读出 `ValidateSignature` 的入口——必须走方法表反查或运行时观察。

### 可用的运行时观察手段（已验证可用）

- DYLD interpose（`__DATA,__interpose` 段）在该客户端上有效：替换 `read` 能拿到 604 字节 ULF 读取栈；
  直接重定义符号无效，必须用 interpose 段。
- `_dyld_get_image_vmaddr_slide` 取主镜像 slide（例：0x3bc000），VA = 运行时地址 − slide，
  文件偏移 = VA − 0x100000000（`__TEXT` fileoff=0）。
- 抓取方法：先带 dylib 起 client（它会建 `/tmp/Unity-LicenseClient-<user>.sock`），再跑一次
  `Unity -batchmode -quit -createProject ...` 让编辑器连上它并触发 ULF 解析；客户端日志里就会出现上面的托管栈。

### 动态观测工作台（已可复用）

拦截库（`__DATA,__interpose` 段，直接重定义符号无效）：

```c
typedef struct { const void *replacement; const void *replacee; } interpose_t;
__attribute__((used)) static const interpose_t interposers[]
    __attribute__((section("__DATA,__interpose"))) = {
        { (const void *)my_read, (const void *)read },
};
```

流程：

1. `DYLD_INSERT_LIBRARIES=interpose.dylib ./Unity.Licensing.Client`（客户端会建
   `/tmp/Unity-LicenseClient-<user>.sock`）；
2. 跑一次 `Unity -batchmode -quit -createProject …`，编辑器连上这个已被拦截的客户端并发起 entitlement 请求；
3. 拦截器把 ULF 读取（604 字节）时的 `backtrace()` 写入日志；
4. `VA = runtime_addr − _dyld_get_image_vmaddr_slide(主镜像)`，`file_off = VA − 0x100000000`。

实测得到的结论：

- 读 ULF 的调用链是 **Stream/FileStream 包装层**（`0x1002d4d40`、`0x1002db140`、`0x10024de00` …），
  不是 `XmlReader.Read`，所以从这里拿不到 `ValidateSignature` 的调用点；
- 这些包装层大量使用 **间接调用**：`adrp x11, …; add x11, …; ldr x16, [x11]; blr x16`
  （槽位在 `0x1013exxx` / `0x1013fxxx` 一带）以及 `ldr x1,[x0,#0x10]; blr x1` 的接口分发，
  因此「按 `bl` 反查目标」不成立；
- 托管字符串字面量（含 `The digital signature is invalid.`）**没有** `adrp/add` 直接引用，
  是启动期由运行时从字符串 blob 水合出来的，代码只引用水合后的槽位。

### 调用图求解（本轮新增，工具已跑通）

用运行时内存快照把间接调用解析成直接调用，再按 traceback 的三个已知偏移做三层链匹配：

1. `dump.dylib`：启动后 5s 用 `mach_vm_region` + `mach_vm_read_overwrite` 把主镜像
   `[0x100000000, 0x100000000+0x1b00000)` 整段转储到 `/tmp/dump.bin`（27MB，含 `__DATA` 的
   水合结果），并记录 slide；
2. `chain.py`：线性反编译 `__managedcode`（10.3MB），跟踪 `adrp/add/ldr` 的寄存器常量，
   遇到 `ldr xD,[已知页]` 时从 dump 里取该槽位的真实指针（减 slide 得 VA），
   从而把 `blr` 也解析成目标；得到 251,795 个调用点、228,351 个可解析目标；
3. 反向索引后按 `Read + 0xd8`、`Parse + 0xbc`、`Map + 0x12c` 三层链搜索。

结果收敛到唯一一组 `Map 0x100122f3c…`、`Parse 0x1001548c0`、`Read 0x100153f10`、
候选 `V = 0x100159f10`（`Read+0xdc` 返回地址正好对上 traceback 的 `Read + 0xdc`）。

**但这个候选不是 ValidateSignature**，已用两种方式反证：

- 把 `0x159f10` 改成 `brk #1`：客户端在**打印 `Starting IPC server` 之前**就 trap（exit 133），
  说明它位于启动早期路径，不是 ULF 解析那一段；
- 改成 `mov w0,#1; ret`：客户端启动即 `System.Console.EnsureInitialized` NRE（写 stderr 失败）；
  改成 `mov w0,#0; ret`：启动正常，但日志仍是 `The digital signature is invalid.`。

结论：traceback 里的 `+ 0xdc` 属于**泛型实例化后的一份 `XmlReader<T>.Read`**，
和静态匹配到的那份不是同一份代码；同一套调用形状在多个实例化里重复，偏移会撞车。
另外 `0x159f10` 的入口形态（`ldr x1,[x19]; ldr x1,[x1,#0x40]; blr x1; cbz w1` 后接固定数据
`0x10142bcc0` 抛错）是典型的“虚拟调用 + 判空抛错”包装，不是 XML 验签本体。

### 动态注入的可用条件（踩坑记录）

`DYLD_INSERT_LIBRARIES` 对这份客户端**不是永远可用**：

- 12:25 / 12:35 两次成功（interpose 抓栈、dump 内存）时，客户端是被 `codesign --deep --sign -`
  重签过的 ad-hoc 版本；
- 12:47 从备份还原后再注入，客户端不再加载任何插入库（连只写一个文件的探针 dylib 都不执行），
  去掉 `com.apple.provenance` 也不生效（该 xattr 受保护，删不掉）。

结论：做动态分析前，必须先把 `UnityLicensingClient.app` 重新 ad-hoc 签名（`codesign --force --deep
--sign -`），并在同一次实验里完成注入；还原原始二进制后注入会静默失效，容易误判为“函数没被调用”。

### 本轮新发现：SecKeyVerifySignature 与第二条调用路径

1. 客户端跑 ULF/entitlement 解析时，stderr 会打出 Security.framework 的
   `SecKeyVerifySignature failed: ... RSA signature verification failed, no match`，
   说明验签最终落到 `SecKeyVerifySignature`。
   - 符号位置：`otool -Iv` → `__stubs` 条目 `0x100a83e40`、`__got` 槽 `0x101118340`；
   - 把 `0x100a83e40` 存根改成 `mov w0,#1; ret`（`20 00 80 52 C0 03 5F D6`）并重签后，
     stderr 里的 `SecKeyVerifySignature failed` 消失（存根生效），但 ULF 仍然报
     `The digital signature is invalid.`，且调用点 `0x100032e28 / 0x100032ecc`（AppleCrypto
     原生 helper 内）根本没有被命中。
   - 结论：**ULF 在验签之前就先在摘要/结构比较上失败了**，单纯伪造 RSA 验签结果不够。
2. 编辑/伪造 entitlement 文件同样不可行：把 `~/Library/Unity/licenses/UnityEntitlementLicense.xml`
   里的 `ebgamedeMini` 改成当前主机名后，客户端立刻报签名无效——这个文件也有签名校验，
   走的还是同一个 `ValidateSignature`（调用链
   `UnityLicense..ctor + 0xf0 → UnityLicense.ReadAndParseLicense + 0x6c → XmlReader.Read → ValidateSignature`）。
   该 XML 原本能解析成功，只是 `ContextValidator` 因主机名/域不一致而拒绝授权。
3. 因此唯一可行点依旧是 `XmlExtensions.ValidateSignature` 本体（两个入口都汇到它）。
   新增的非泛型调用链线索（`UnityLicense..ctor +0xf0 → ReadAndParseLicense +0x6c`）比泛型
   `XmlReader<T>.Read` 更可靠，但按 `+0xf0/+0x6c` 偏移反查仍会撞出上千个误报，
   需要配合“函数大小 / 是否调用 crypto”等额外约束。

### 本轮实测：289 个候选全部落空（含结论修正）

把"`Read + 0xd8` / `ReadAndParseLicense + 0x68` 两个已知调用点"作为约束，并要求调用者起点通过
函数头启发式（`stp x29,x30,[sp,#-N]!` / `sub sp,sp,#N`，且前一条是 `ret`/填充），得到 127 + 162 个候选。
用「静态打 `mov w0,#1; ret` → ad-hoc 重签副本 → 独立跑 2s → 检查新增日志」的批处理逐个验证：

- 日志里 `Exception caught while parsing license /Library/.../Unity_lic.ulf` 共出现 232 次，
  证明绝大多数候选运行时确实走到了 ULF 解析（oracle 有效）；
- 289 个候选**全部**没有把结果变成 `Successfully parsed (ULF)`；其中 2 个候选（`0x1006e8480`、
  `0x100293320`）会让客户端直接 abort，说明它们确实在启动路径上，但不是验签函数。

结论：**"调用点 = 方法起点 + (traceback 偏移 - 4)"** 这个假设在这份 AOT 代码里不成立（或函数起点识别
不准）。下一步必须先从运行时取到 `ValidateSignature` 的绝对地址，而不是继续用偏移反推：

1. lldb 在 `write` 命中 `digital signature` 时停下，从当前线程栈上定位 `InvalidDataException` 实例，
   读它内部捕获的 stack-trace IP 数组（Native AOT 会为异常保存 IP 列表，才可能打印出方法名+偏移）；
2. 或者断在 `SecKeyVerifySignature` 的调用点 `0x100032e28`（ULF 解析确实会走到，stderr 有 seckey 记录），
   取 `lr` 的托管返回地址，向上回溯到调用它的托管方法，再用该地址减 0x590 得到方法起点。

### 另一个反证：SecKeyVerifySignature 不走静态存根

恢复原版客户端后，用 lldb 在 `0x100a83e40`（`otool -Iv` 标为 `_SecKeyVerifySignature` 的 `__stubs` 条目）
和原生 helper 里的两个 `bl` 点（`0x100032e28 / 0x100032ecc`）下断点，运行时 stderr 明确打出
`SecKeyVerifySignature failed: ...`，但这三个地址**一个都没命中**。
说明验签调用不是走静态 stub/`bl`（很可能是 `dlsym`/函数指针间接触发），
因此"改存根让验签恒真"这条捷径不成立，真正的门禁仍只能落在 `ValidateSignature` 本体上。

### 下一步（已收敛）

优先级 1：**从运行时异常对象里取 IP**。客户端抛异常时会捕获托管栈（所以日志能打印方法名+偏移），
在 `write` 命中 `digital signature` 的那次停下，从当前线程栈上找到 `InvalidDataException` 实例，
读它内部的 stack-trace 数组即可直接得到 `ValidateSignature + 0x590` 的绝对地址 —— 这是唯一不需要
再猜实例化的路径。

优先级 2：编辑器侧 entitlement 门禁。已定位 `com.unity.editor.ui`（`0x10447e5b7`）、
`com.unity.editor.headless`（`0x10447e5cb`）、`No valid Unity Editor license found`（`0x10465725d`）
及其 xref（`0x100800260/0x10080027c`、`0x103297818`），这些是编辑器侧构建 entitlement 名称表的位置；
顺着表的使用点可找到“拿不到 entitlement 就退出 198”的判定分支。

### 早期推测（保留）

1. 用 `__modules` → TypeManager 的 `ReflectionMethodInfo`（名字 `ValidateSignature`，文件 0xb6fb2d 附近）
   找到对应 MethodDesc；Native AOT 的 MethodDesc 保存到原生代码的相对偏移，可直接算出入口 VA；
2. 入口打 `mov w0,#1; ret`（`0x52800020 0xd65f03c0`）或 `ret`（`0xd65f03c0`）；
3. 用同一条 `-createProject` 流程验证：日志应从 `The digital signature is invalid.` 变成
   `Successfully parsed (ULF) license`。

## AOT client 侧（6000.3.28+）早期记录

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
