//! 就地 IL 补丁：对 `Unity.Licensing.EntitlementResolver.dll`（任意 LocalIPC 发行线）
//! 的 `ValidateSignature` 方法做字节级补丁，绕过 ULF 签名校验。
//!
//! 与"预编译补丁 DLL"方案（dnlib 重写后替换）的区别：
//! - 本方案**零改动**程序集身份（Assembly Name/Version/PublicKeyToken、引用运行时、
//!   类型/方法表、局部变量、异常处理子句全部原样保留），只改写方法体里 4 条指令；
//! - 因此不依赖具体 LocalIPC 线（1.17.x、1.18+、未来的 1.19/… 通吃），只要
//!   `ValidateSignature` 的 IL 结构未被改型即可命中；
//! - 命中失败时返回 Err，由调用方回退到按线匹配的预编译补丁 DLL。
//!
//! ## 补丁依据（dnlib 逆向所得，见 `tools/patchresolver/Program.cs`）
//!
//! 原代码（13 字节，栈平衡：brtrue.s 消费 bool，throw 消费异常）：
//! ```text
//! brtrue.s 0x2D xx                          ; 2 字节
//! ldstr    0x72 t0 t1 t2 t3                 ; 5 字节，token → #US == "The digital signature is invalid."
//! newobj   0x73 m0 m1 m2 m3                 ; 5 字节
//! throw    0x7A                             ; 1 字节（0x2A 是 ret，勿混淆）
//! ```
//! 补丁后（仍 13 字节，栈等价：pop 消费 bool）：
//! ```text
//! pop  0x26                                ; 1 字节
//! nop  0x00 × 12                            ; 12 字节
//! ```
//! 执行流不再进入错误路径，而是穿过原错误块落入成功返回路径；方法体长度不变，
//! 后续指令、分支偏移、异常处理子句全部不受影响。
//!
//! ## ReadyToRun 关键处理
//!
//! Unity 的 resolver 是 **ReadyToRun 镜像**（CLI 头 `ManagedNativeHeader` 指向
//! `RTR` 原生代码）。运行时优先执行 R2R 原生代码，**只改 IL 而不清 native header
//! 时补丁永远不生效**（dnlib 看 IL 会"验证通过"、运行期行为不变）。
//! 本模块在改完 IL 后会把 `ManagedNativeHeader`（RVA + Size）清零，使运行时按
//! 纯 IL 加载并基于补丁后的 IL 重新 JIT。预编译（dnlib 重写）产物本就无
//! native header，该操作幂等无害。
//!
//! ## 匹配精度
//!
//! 不解析 MethodDef 表（coded index 规则复杂且易错），而是对整文件扫描
//! `28 D2 ?? 72 t0 t1 t2 t3 73 m0 m1 m2 m3 2A` 模式，并用 `ldstr` 的 token 反查
//! `#US` 堆确认字符串一字不差 —— 该组合在 500KB 级 DLL 中只会命中真实错误路径
//! （等价于 dnlib 版本按名称+字符串+指令序列的三重约束；命中数会随拷贝字节模式
//! 概率性出现，但假阳性概率约为 2^-108，可忽略）。补丁状态幂等探测为
//! `26` + 12×`00` 窗口。

use std::fmt;

/// 补丁结果报告。
#[derive(Debug, Clone)]
pub struct PatchReport {
    /// 本次实际打上的补丁数（原始模式命中并改写）。
    pub patched_sites: usize,
    /// 已处于补丁状态（`pop` + 3×`nop` 及以上）的站点数，用于幂等探测。
    pub already_patched_sites: usize,
    /// 原文件是否为 ReadyToRun（存在 ManagedNativeHeader）且本次已将其清零。
    pub r2r_native_header_cleared: bool,
}

impl fmt::Display for PatchReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "sites patched: {}, already patched: {}, r2r cleared: {}",
            self.patched_sites, self.already_patched_sites, self.r2r_native_header_cleared
        )
    }
}

/// 目标错误字符串（33 个 UTF-16 字符，66 字节）。
const TARGET_STR: &str = "The digital signature is invalid.";

/// 尝试就地补丁 resolver DLL。
///
/// - `Ok((patched_data, report))`：`patched_sites > 0` 表示已改写；
///   `patched_sites == 0 && already_patched_sites > 0` 表示已处于补丁状态
///   （幂等命中，返回值与输入一致）。
/// - `Err`：不是托管程序集 / 元数据损坏 / 未命中任何模式，
///   调用方应回退到预编译补丁 DLL。
pub fn patch_resolver(data: &[u8]) -> Result<(Vec<u8>, PatchReport), String> {
    patch_resolver_impl(data, true)
}

/// 与 `patch_resolver` 相同，但**保留** ReadyToRun 头（不清 `ManagedNativeHeader`）。
///
/// macOS 上必须用这个变体：.NET 宿主在启动时按 PE 的 R2R 头决定 TPA 条目，清掉
/// `ManagedNativeHeader` 后程序集会被直接判为不可加载（表现为
/// `FileNotFoundException: Could not load file or assembly '...EntitlementResolver'`）。
/// 因此 macOS 侧保留 R2R 头、只改 IL，并让 licensing client 以
/// `DOTNET_ReadyToRun=0` 启动，使运行时不走原生代码（见 patcher 里的 launcher shim）。
pub fn patch_resolver_keep_r2r(data: &[u8]) -> Result<(Vec<u8>, PatchReport), String> {
    patch_resolver_impl(data, false)
}

fn patch_resolver_impl(data: &[u8], strip_r2r: bool) -> Result<(Vec<u8>, PatchReport), String> {
    let md = ManagedMetadata::parse(data)?;

    let mut out = data.to_vec();
    let mut report = PatchReport {
        patched_sites: 0,
        already_patched_sites: 0,
        r2r_native_header_cleared: false,
    };

    let mut i = 0usize;
    while i + 13 <= data.len() {
        if data[i] == 0x2D
            && data[i + 2] == 0x72
            && data[i + 7] == 0x73
            && data[i + 13 - 1] == 0x7A // throw（0x2A 是 ret）
        {
            let token = u32::from_le_bytes([
                data[i + 3],
                data[i + 4],
                data[i + 5],
                data[i + 6],
            ]) & 0x00FF_FFFF;
            if md.us_string(token)? == Some(()) {
                // brtrue.s → pop；其余 12 字节 → nop
                out[i] = 0x26;
                for b in &mut out[i + 1..i + 13] {
                    *b = 0x00;
                }
                report.patched_sites += 1;
                i += 13;
                continue;
            }
        }
        // 幂等探测：补丁产物形态有两种——就地 13 字节（pop + 12×nop）与
        // dnlib 重写 4 字节（pop + 3×nop），后续指令均为原 throw 之后的 ret(0x2A)
        // 或继续填充的 nop(0x00)。
        if data[i] == 0x26
            && data[i + 1] == 0x00
            && data[i + 2] == 0x00
            && data[i + 3] == 0x00
            && (data[i + 4] == 0x00 || data[i + 4] == 0x2A)
        {
            report.already_patched_sites += 1;
        }
        i += 1;
    }

    if report.patched_sites == 0 && report.already_patched_sites == 0 {
        return Err(
            "ValidateSignature IL pattern not found; resolver variant not recognized".into(),
        );
    }

    // ReadyToRun 处理：Unity 的 resolver 是 R2R 镜像（CLI 头 ManagedNativeHeader 指向
    // "RTR" 原生代码），运行时优先执行原生代码而非 IL —— 只改 IL 而不清 native header
    // 时，补丁永远不会生效（dnlib 验证通过但运行期行为不变）。
    // 清零 CLI 头 ManagedNativeHeader（RVA + Size）后，运行时按纯 IL 加载 → 从补丁后的
    // IL 重新 JIT。预编译（dnlib 重写）产物本来就无 native header，此操作是无害幂等。
    let (cli_flags, mnh_rva, mnh_size) = md.cli_header_fields();
    if strip_r2r && (mnh_rva != 0 || mnh_size != 0) {
        cli_header_clear_managed_native_header(&mut out, md.cli_off);
        report.r2r_native_header_cleared = true;
    }
    let _ = cli_flags; // 保留 Flags 不动

    Ok((out, report))
}

// ---------------------------------------------------------------------------
// ECMA-335 元数据根 + #US 堆解析（最小子集：足够校验 ldstr 字符串）
// ---------------------------------------------------------------------------

struct ManagedMetadata<'a> {
    data: &'a [u8],
    us_off: usize,
    us_size: usize,
    /// CLI 头（COM descriptor）在文件中的偏移。
    cli_off: usize,
}

impl<'a> ManagedMetadata<'a> {
    /// CLI 头字段：Flags、ManagedNativeHeader RVA/Size。
    fn cli_header_fields(&self) -> (u32, u32, u32) {
        let c = self.cli_off;
        (
            le_u32(self.data, c + 16),
            le_u32(self.data, c + 64),
            le_u32(self.data, c + 68),
        )
    }
    fn parse(data: &'a [u8]) -> Result<Self, String> {
        if data.len() < 0x40 || &data[0..2] != b"MZ" {
            return Err("not a PE file".into());
        }
        let pe_off = le_u32(data, 0x3C) as usize;
        if data.len() < pe_off + 24 || &data[pe_off..pe_off + 4] != b"PE\0\0" {
            return Err("invalid PE header".into());
        }
        let opt_size = le_u16(data, pe_off + 20) as usize;
        let opt = pe_off + 24;
        let dir_base = match le_u16(data, opt) {
            0x10B => opt + 96,
            0x20B => opt + 112,
            _ => return Err("unsupported PE optional header magic".into()),
        };
        let com_rva = le_u32(data, dir_base + 14 * 8);
        if com_rva == 0 {
            return Err("not a managed assembly (no COM descriptor)".into());
        }

        // 节表：RVA → 文件偏移
        let num_sections = le_u16(data, pe_off + 6) as usize;
        let sec_base = opt + opt_size;
        let cli_off = {
            let mut cli = None;
            for s in 0..num_sections {
                let off = sec_base + s * 40;
                if data.len() < off + 40 {
                    return Err("truncated section table".into());
                }
                let va = le_u32(data, off + 12);
                let span = le_u32(data, off + 8).max(le_u32(data, off + 16));
                let raw = le_u32(data, off + 20);
                if com_rva >= va && com_rva < va + span {
                    cli = Some((com_rva - va + raw) as usize);
                }
            }
            cli.ok_or("COM descriptor RVA not mapped")?
        };
        if cli_off + 16 > data.len() {
            return Err("COM descriptor out of range".into());
        }
        let md_rva = le_u32(data, cli_off + 8);
        let mut meta_off = None;
        for s in 0..num_sections {
            let off = sec_base + s * 40;
            let va = le_u32(data, off + 12);
            let span = le_u32(data, off + 8).max(le_u32(data, off + 16));
            let raw = le_u32(data, off + 20);
            if md_rva >= va && md_rva < va + span {
                meta_off = Some((md_rva - va + raw) as usize);
            }
        }
        let meta_off = meta_off.ok_or("metadata RVA not mapped")?;
        if data.len() < meta_off + 16 || le_u32(data, meta_off) != 0x424A_5342 {
            return Err("invalid metadata root".into());
        }

        // 流头：偏移 + 大小 + 名称（4 字节对齐）
        let ver_len = le_u32(data, meta_off + 12) as usize;
        let mut pos = meta_off + 16 + align4(ver_len);
        let stream_count = le_u16(data, pos + 2) as usize;
        pos += 4;

        let mut us_off = None;
        let mut us_size = 0usize;
        for _ in 0..stream_count {
            if pos + 8 > data.len() {
                return Err("stream header truncated".into());
            }
            let off = le_u32(data, pos) as usize;
            let size = le_u32(data, pos + 4) as usize;
            let mut n = pos + 8;
            while n < data.len() && data[n] != 0 {
                n += 1;
            }
            let name = &data[pos + 8..n];
            pos = meta_off + align4(n - meta_off + 1);
            if name == b"#US" {
                us_off = Some(meta_off + off);
                us_size = size;
            }
        }
        let us_off = us_off.ok_or("no #US stream")?;
        Ok(Self {
            data,
            us_off,
            us_size,
            cli_off,
        })
    }

    /// 读取 #US 条目并判断是否为目标错误字符串。
    /// 条目格式：压缩长度 + UTF-16LE（不计尾部 0xFF 填充）。
    fn us_string(&self, idx: u32) -> Result<Option<()>, String> {
        if idx == 0 {
            return Ok(None);
        }
        let p = self.us_off + idx as usize;
        if p >= self.us_off + self.us_size {
            return Ok(None); // 索引越界（也可能因为堆用了 4 字节索引而 idx 语义不同）
        }
        let b0 = self.data[p];
        let (len, data_start) = if b0 & 0x80 == 0 {
            (b0 as usize, p + 1)
        } else if b0 & 0x40 == 0 {
            if p + 2 > self.data.len() {
                return Ok(None);
            }
            (((b0 & 0x3F) as usize) << 8 | self.data[p + 1] as usize, p + 2)
        } else {
            if p + 5 > self.data.len() {
                return Ok(None);
            }
            (
                (((b0 & 0x1F) as usize) << 24)
                    | ((self.data[p + 1] as usize) << 16)
                    | ((self.data[p + 2] as usize) << 8)
                    | (self.data[p + 3] as usize),
                p + 4,
            )
        };
        // 目标：33 个 UTF-16 字符 = 66 字节。实测该堆存在多种编码变体：
        // idx=15561 条目 len=66（无终止字节）；idx=4179 条目 len=67（66 字节 + 0x00）；
        // 规范允许 0xFF 填充。策略：前 66 字节精确匹配，尾部只允许 0x00/0xFF。
        let exp_len = TARGET_STR.encode_utf16().count() * 2;
        if len < exp_len || len > exp_len + 2 {
            return Ok(None);
        }
        if data_start + len > self.us_off + self.us_size || data_start + len > self.data.len() {
            return Ok(None);
        }
        let entry = &self.data[data_start..data_start + len];
        let mut exp = Vec::with_capacity(exp_len);
        for u in TARGET_STR.encode_utf16() {
            exp.extend_from_slice(&u.to_le_bytes());
        }
        if entry[..exp_len] != exp[..] {
            return Ok(None);
        }
        if entry[exp_len..].iter().any(|&b| b != 0x00 && b != 0xFF) {
            return Ok(None);
        }
        Ok(Some(()))
    }
}

/// 清零 CLI 头的 ManagedNativeHeader（RVA @+64 u32、Size @+68 u32）：
/// ReadyToRun 镜像必须清除，否则运行时继续执行旧的 R2R 原生代码。
fn cli_header_clear_managed_native_header(out: &mut [u8], cli_off: usize) {
    let b = &mut out[cli_off..cli_off + 72];
    for i in [64usize, 68] {
        b[i] = 0;
        b[i + 1] = 0;
        b[i + 2] = 0;
        b[i + 3] = 0;
    }
}

/// 小端读取辅助。
fn le_u16(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}

fn le_u32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([
        data[off],
        data[off + 1],
        data[off + 2],
        data[off + 3],
    ])
}

fn align4(n: usize) -> usize {
    (n + 3) & !3
}

// ---------------------------------------------------------------------------
// 测试：需要真实 resolver DLL，环境变量指路（cargo test -- --ignored 本地手动验证）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires real resolver DLL paths via env"]
    fn patch_real_resolver_inplace() {
        let env = "UNIFREE_TEST_RESOLVER_ORIGINAL";
        let data = std::fs::read(
            std::env::var(env)
                .unwrap_or_else(|_| panic!("set {env} to an original resolver DLL")),
        )
        .expect("read resolver");
        let (patched, report) = patch_resolver(&data).expect("patch should succeed");
        let (patched, report) = patch_resolver(&data).expect("patch should succeed");
        eprintln!("original resolver: {report}");
        assert!(
            report.patched_sites > 0,
            "expected original pattern sites, got {report}"
        );

        // 幂等：再补一次应识别为已补丁，且不改变内容
        let (_, report2) = patch_resolver(&patched).expect("re-patch should succeed");
        eprintln!("re-patch: {report2}");
        assert!(report2.patched_sites == 0, "{report2}");
        assert!(report2.already_patched_sites > 0, "{report2}");

        // 确定性 + 大小不变（就地补丁的硬性约束）
        let (patched2, _) = patch_resolver(&data).unwrap();
        assert_eq!(patched, patched2);
        assert_eq!(patched.len(), data.len());

        // 可选：把补丁产物写出（供 tools/verify-resolver 等外部验证）
        if let Ok(out) = std::env::var("UNIFREE_TEST_OUTPUT") {
            std::fs::write(&out, &patched).expect("write patched output");
            eprintln!("wrote patched output to {out}");
        }
    }

    #[test]
    #[ignore = "requires a resolver already patched (precompiled or in-place) via env"]
    fn detect_already_patched_resolver() {
        let env = "UNIFREE_TEST_RESOLVER_PATCHED";
        let data = std::fs::read(
            std::env::var(env)
                .unwrap_or_else(|_| panic!("set {env} to an already-patched resolver DLL")),
        )
        .expect("read resolver");
        let (out, report) = patch_resolver(&data).expect("parse should succeed");
        eprintln!("pre-patched resolver: {report}");
        assert!(
            report.patched_sites == 0 && report.already_patched_sites > 0,
            "{report}"
        );
        assert_eq!(out, data, "idempotent: no bytes should change");
    }
}
