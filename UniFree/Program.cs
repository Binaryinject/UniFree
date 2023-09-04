using System.Buffers.Text;
using System.Diagnostics;
using System.Reflection;
using System.Security.Principal;
using System.Text;
using zFramework.IO;

static void RelaunchIfNotAdmin() {
    if (!RunningAsAdmin()) {
        Console.WriteLine("Running as admin required!");
        ProcessStartInfo proc = new ProcessStartInfo();
        proc.UseShellExecute = true;
        proc.WorkingDirectory = Environment.CurrentDirectory;
        proc.FileName = "UniFree.exe";
        proc.Verb = "runas";
        try {
            Process.Start(proc);
            Environment.Exit(0);
        }
        catch (Exception) {
            Console.WriteLine("请右键以管理员身份运行！");
            Environment.Exit(0);
        }
    }
}

static bool RunningAsAdmin() {
    WindowsIdentity id = WindowsIdentity.GetCurrent();
    WindowsPrincipal principal = new WindowsPrincipal(id);

    return principal.IsInRole(WindowsBuiltInRole.Administrator);
}

static IEnumerable<long> IndexesOf(byte[] source, byte[] pattern, int start = 0) {
    if (source == null) {
        throw new ArgumentNullException(nameof(source));
    }
    if (pattern == null) {
        throw new ArgumentNullException(nameof(pattern));
    }
    long valueLength = source.LongLength;
    long patternLength = pattern.LongLength;
    if (valueLength == 0 || patternLength == 0 || patternLength > valueLength) {
        yield break;
    }
    var badCharacters = new long[256];
    for (var i = 0; i < 256; i++) {
        badCharacters[i] = patternLength;
    }
    var lastPatternByte = patternLength - 1;
    for (long i = 0; i < lastPatternByte; i++) {
        badCharacters[pattern[i]] = lastPatternByte - i;
    }
    long index = start;
    while (index <= valueLength - patternLength) {
        for (var i = lastPatternByte; source[index + i] == pattern[i]; i--) {
            if (i == 0) {
                yield return index;
                break;
            }
        }
        index += badCharacters[source[index + lastPatternByte]];
    }
}
string ToHexStrFromByte(IEnumerable<byte> byteDatas) {
    var builder = new StringBuilder();
    foreach (var t in byteDatas) {
        builder.Append($"{t:X2}");
    }

    return builder.ToString().Trim();
}

const string ulf = "PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iVVRGLTgiPz4KPHJvb3Q+CiAgICA8TGljZW5zZSBpZD0iVGVybXMiPgogICAgICAgIDxOb0hhcmR3YXJlQ2hlY2sgVmFsdWU9InRydWUiLz4KICAgICAgICA8TWFjaGluZUJpbmRpbmdzPgogICAgICAgICAgICA8QmluZGluZyBLZXk9IjEiIFZhbHVlPSIwMDAwMDAwMC0wMDAwLTAwMDAtMDAwMC0wMDAwMDAwMDAwMDAiLz4KICAgICAgICAgICAgPEJpbmRpbmcgS2V5PSIyIiBWYWx1ZT0iMDAwMDAwMDAwMDAwIi8+CiAgICAgICAgPC9NYWNoaW5lQmluZGluZ3M+CiAgICAgICAgPFNlcmlhbEhhc2ggVmFsdWU9IjAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAiLz4KICAgICAgICA8RmVhdHVyZXM+CiAgICAgICAgICAgIDxGZWF0dXJlIFZhbHVlPSIwIi8+CiAgICAgICAgICAgIDxGZWF0dXJlIFZhbHVlPSIyIi8+CiAgICAgICAgICAgIDxGZWF0dXJlIFZhbHVlPSI0Ii8+CiAgICAgICAgICAgIDxGZWF0dXJlIFZhbHVlPSI5Ii8+CiAgICAgICAgICAgIDxGZWF0dXJlIFZhbHVlPSIxMyIvPgogICAgICAgICAgICA8RmVhdHVyZSBWYWx1ZT0iMjAiLz4KICAgICAgICAgICAgPEZlYXR1cmUgVmFsdWU9IjIxIi8+CiAgICAgICAgICAgIDxGZWF0dXJlIFZhbHVlPSIyMiIvPgogICAgICAgICAgICA8RmVhdHVyZSBWYWx1ZT0iMzAiLz4KICAgICAgICAgICAgPEZlYXR1cmUgVmFsdWU9IjM5Ii8+CiAgICAgICAgICAgIDxGZWF0dXJlIFZhbHVlPSI0MCIvPgogICAgICAgICAgICA8RmVhdHVyZSBWYWx1ZT0iNjAiLz4KICAgICAgICAgICAgPEZlYXR1cmUgVmFsdWU9IjY1Ii8+CiAgICAgICAgPC9GZWF0dXJlcz4KICAgICAgICA8RGV2ZWxvcGVyRGF0YSBWYWx1ZT0iQVFBQUFEQXdMVEF3TURBdE1EQXdNQzB3TURBd0xUQXdNREF0TURBd01BPT0iLz4KICAgICAgICA8U2VyaWFsTWFza2VkIFZhbHVlPSIwMC0wMDAwLTAwMDAtMDAwMC0wMDAwLVhYWFgiLz4KICAgICAgICA8TGljZW5zZVZlcnNpb24gVmFsdWU9IjYueCIvPgogICAgICAgIDxDbGllbnRQcm92aWRlZFZlcnNpb24gVmFsdWU9IjIwMTcuMi4wIi8+CiAgICAgICAgPEFsd2F5c09ubGluZSBWYWx1ZT0iZmFsc2UiLz4KICAgIDwvTGljZW5zZT4KICAgIDxTaWduYXR1cmUgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvMDkveG1sZHNpZyMiPgogICAgICAgIDxTaWduZWRJbmZvIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwLzA5L3htbGRzaWcjIj4KICAgICAgICAgICAgPENhbm9uaWNhbGl6YXRpb25NZXRob2QgQWxnb3JpdGhtPSJodHRwOi8vd3d3LnczLm9yZy9UUi8yMDAxL1JFQy14bWwtYzE0bi0yMDAxMDMxNSNXaXRoQ29tbWVudHMiLz4KICAgICAgICAgICAgPFNpZ25hdHVyZU1ldGhvZCBBbGdvcml0aG09Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvMDkveG1sZHNpZyNyc2Etc2hhMSIvPgogICAgICAgICAgICA8UmVmZXJlbmNlIFVSST0iI1Rlcm1zIj4KICAgICAgICAgICAgICAgIDxUcmFuc2Zvcm1zPgogICAgICAgICAgICAgICAgICAgIDxUcmFuc2Zvcm0gQWxnb3JpdGhtPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwLzA5L3htbGRzaWcjZW52ZWxvcGVkLXNpZ25hdHVyZSIvPgogICAgICAgICAgICAgICAgPC9UcmFuc2Zvcm1zPgogICAgICAgICAgICAgICAgPERpZ2VzdE1ldGhvZCBBbGdvcml0aG09Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvMDkveG1sZHNpZyNzaGExIi8+CiAgICAgICAgICAgICAgICA8RGlnZXN0VmFsdWU+N2V2bGZaUWdyTGZONm5lVHRodGoxMS9STlpBPTwvRGlnZXN0VmFsdWU+CiAgICAgICAgICAgIDwvUmVmZXJlbmNlPgogICAgICAgIDwvU2lnbmVkSW5mbz4KICAgICAgICA8U2lnbmF0dXJlVmFsdWU+WmJUMUowLytDWEpsNnZLdTJqK2FkaFJTNFRpSTFoOHFONkUrRHdWRGdjTWRaNThwY01FSks4RG5KVVRYZEFQZTArbVJCbFU0VWpvMTRWMVlYSjVNVk9zMWJ2UUpUcndlU25EKytPVHdOMU8zZ1luTGljSnhzdUVISzhWYUlLRVlMMFZSOXNkem1aSlBqV0lLN0lZczhuSnluSnI3L1ZUa2RGeTUzY0doa0ZrNEdSYTF5WGdrMTd6bGEyVDloVFhVRks4anJKOEovQ3dGQUdlbG0rQlp3NWxGUGxKTUQ2djlJM3lwdDNmdUcwMThyTGxIenRzY3BiTUVmajI2aVcwckhJT01LQnl6NTQzZkVQZ2RaVEladG8zU0RTTVNacEllaS9vUE9ORW5hemtFT09zdVAxby9zVDZ6UEdQU1RJbXA4Z3NXZEw3cks4TEphdG1oV1piZ1N3PT08L1NpZ25hdHVyZVZhbHVlPgogICAgPC9TaWduYXR1cmU+Cjwvcm9vdD4=";
const string pemBegin = @"-----BEGIN CERTIFICATE-----
";
const string pemEnd = @"
-----END CERTIFICATE-----";
const string modPem = @"MIIE7zCCA9egAwIBAgIUUCAbT5WTG+L4uTES4NbaOwvc4L8wDQYJKoZIhvcNAQEFBQAwgfMxCzAJ
BgNVBAYTAkRLMRMwEQYDVQQIDApDb3BlbmhhZ2VuMRMwEQYDVQQHDApDb3BlbmhhZ2VuMR8wHQYD
VQQKDBZVbml0eSBUZWNobm9sb2dpZXMgQXBzMUkwRwYDVQQLDEAuLi4uLi4uLi4uLi4uLi4uLi4u
Li4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uMQswCQYDVQQDDAJJ
VDEgMB4GCSqGSIb3DQEJARYRYWRtaW5AdW5pdHkzZC5jb20xHzAdBgNVBCkMFi4uLi4uLi4uLi4u
Li4uLi4uLi4uLi4wHhcNMjMwNDI2MTIxMDQzWhcNMjQwNDI1MTIxMDQzWjCB8zELMAkGA1UEBhMC
REsxEzARBgNVBAgMCkNvcGVuaGFnZW4xEzARBgNVBAcMCkNvcGVuaGFnZW4xHzAdBgNVBAoMFlVu
aXR5IFRlY2hub2xvZ2llcyBBcHMxSTBHBgNVBAsMQC4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4u
Li4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4uLi4xCzAJBgNVBAMMAklUMSAwHgYJ
KoZIhvcNAQkBFhFhZG1pbkB1bml0eTNkLmNvbTEfMB0GA1UEKQwWLi4uLi4uLi4uLi4uLi4uLi4u
Li4uLjCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAL3t/3ossioWbzMe6SK13Zn7rskP
kLtNJjlNUbFRxz7IzJokoG387NSXOlOAn2qEHIRpDNsxDTH20fkLzYBwe0Tr5a2u9l+DOWpRnt7J
zMJaTBWiWsrLnZht5ePRj7Vn7c25qm7Pdq9iuYr0zKmqEW3+eNH7a6PHgQyJRLkk/zuE0drczBkn
REazOmACir9o1gjU/U36FYN+v3r4sELHDKmJ5J+QrxfmxFsXzXuZc8wTrE8pxWXIIWLZRjic/zKx
VQifYUQ9wUhFJuTdByuHKhi37uG8jUNyfPe1PCffQhLRb2pbCw5kjKfkJwJexhd+MBsODFboC3oh
ryD0Vwq3678CAwEAAaN5MHcwDgYDVR0PAQH/BAQDAgWgMB0GA1UdJQQWMBQGCCsGAQUFBwMCBggr
BgEFBQcDBDAJBgNVHRMEAjAAMBwGA1UdEQQVMBOBEWFkbWluQHVuaXR5M2QuY29tMB0GA1UdDgQW
BBQ61G5XMwod4trNVUBXzlbFSdA0yzANBgkqhkiG9w0BAQUFAAOCAQEAImvfvAvV5Yqq2gjF16Sb
y9MGIQiYjyVBIDoHxwXneHCAaxR6IKS+5elwUruJ/D+BK6vK09Ju9DzqLL6yHo/ZqfycfkcAGcxH
/RTOnuz/ZZ2pLHyz0sjvoONQLHNKmYTvMnLvQLiTj71VYyMUQW6EfaFttIDzzW48cUduExH+UIiQ
9OeZmSEsW+NeGB4KejdggVTqJMrPf85kA/8PBxlYlxn6icqDu40T4l1Uc/qDvj8d598MbOD1PBLc
nbAR2VgljJN55v48Fmmwg1g2cIjWQt4jTuJWL67O65S1tKKmlsnEKT+37f2B8l2ijm7/fwA9qI2i
dAUsITaJDHJ+FC6LGg==";

RelaunchIfNotAdmin();

Console.WriteLine($"UniFree v{Assembly.GetEntryAssembly()?.GetName().Version?.ToString()} by binaryinject");
Console.WriteLine("==========================选择文件==========================");
var file = FileDialog.SelectFile("选择Unity.exe", "Unity.exe|Unity.exe");

if (file.Count == 0) {
    Console.WriteLine("未选择Unity.exe");
    Console.ReadKey();
    return;
}

Console.WriteLine($"选择Unity路径：{file[0]}");

var basePath = file[0][.. (file[0].Length - "Unity.exe".Length)];
var dllPath = $@"{basePath}Data\Resources\Licensing\Client\Unity.Licensing.EntitlementResolver.dll";

if (!File.Exists(dllPath)) {
    Console.WriteLine("Unity.Licensing.EntitlementResolver.dll未找到");
    Console.ReadKey();
    return;
}

var allBytes = await File.ReadAllBytesAsync(dllPath);
var beginBytes = Encoding.UTF8.GetBytes(pemBegin);
var endBytes = Encoding.UTF8.GetBytes(pemEnd);
var beginIndex = IndexesOf(allBytes, beginBytes).ToArray();
var endIndex = IndexesOf(allBytes, endBytes).ToArray();
var patched = Encoding.UTF8.GetBytes(modPem);
var patchedIndex = IndexesOf(allBytes, patched).ToArray();
if (patchedIndex.Length > 0 || beginIndex.Length == 0 || endIndex.Length == 0) {
    Console.WriteLine(patchedIndex.Length > 0 ? "已经打过补丁了" : "没有找到补丁点位");
    Console.ReadKey();
    return;
}
Console.WriteLine("==========================开始备份==========================");
var backup = $"{dllPath}.bak";
await File.WriteAllBytesAsync(backup, allBytes);
Console.WriteLine($"备份位置：{backup}");
Console.WriteLine("==========================开始补丁==========================");

if (patched.Length != endIndex[0] - beginIndex[0] - beginBytes.Length) {
    Console.WriteLine("证书长度大小异常");
    Console.ReadKey();
    return;
}

var start = beginIndex[0] + beginBytes.Length;
for (long i = 0; i < patched.Length; i++) {
    allBytes[start + i] = patched[i];
}

await File.WriteAllBytesAsync(dllPath, allBytes);
Console.WriteLine($"补丁位置：{start}");

Console.WriteLine("==========================授权拷贝==========================");
var ulfBytes = Convert.FromBase64String(ulf);
const string ulfDirectory = @"C:\ProgramData\Unity";
const string ulfPath = @"C:\ProgramData\Unity\Unity_lic.ulf";
if (!Directory.Exists(ulfDirectory)) Directory.CreateDirectory(ulfDirectory);
await File.WriteAllBytesAsync(ulfPath, ulfBytes);
if (File.Exists(ulfPath)) {
    Console.WriteLine($"授权已拷贝：{ulfPath}");
}
else {
    Console.WriteLine($"授权拷贝失败，请手动复制目录下Unity_lic.ulf至{ulfPath}");
    await File.WriteAllBytesAsync("Unity_lic.ulf", ulfBytes);
}

Console.WriteLine("破解完成");
Console.ReadKey();