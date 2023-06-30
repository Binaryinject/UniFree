using System.Diagnostics;
using System.Diagnostics.CodeAnalysis;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Principal;
using zFramework.IO;

static void ExecuteCommand(string command) {
    var processInfo = new ProcessStartInfo("cmd.exe", "/c " + command);
    processInfo.CreateNoWindow = true;
    processInfo.UseShellExecute = false;
    processInfo.RedirectStandardError = true;
    processInfo.RedirectStandardOutput = true;

    var process = Process.Start(processInfo);
    if (process != null) {
        process.OutputDataReceived += (_, e) => Console.WriteLine(e.Data);
        process.BeginOutputReadLine();

        process.ErrorDataReceived += (_, e) => Console.WriteLine(e.Data);
        process.BeginErrorReadLine();

        process.WaitForExit();

        //Console.WriteLine("ExitCode: {0}", process.ExitCode);
        process.Close();
    }
}

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

RelaunchIfNotAdmin();
Console.WriteLine("UniFree v1.0 by binaryinject");
Console.WriteLine("==========================选择文件==========================");
var file = FileDialog.SelectFile("选择Unity.exe", "Unity.exe|Unity.exe");

if (file.Count == 0) {
    Console.WriteLine("未选择Unity.exe!");
    Console.ReadKey();
    return;
}

Console.WriteLine($"选择Unity路径：{file[0]}");

Console.WriteLine("==========================开始补丁==========================");

var basePath = file[0][.. (file[0].Length - "Unity.exe".Length)];
var dllPath = $@"{basePath}Data\Resources\Licensing\Client\Unity.Licensing.EntitlementResolver.dll";

if (!File.Exists(dllPath)) {
    Console.WriteLine("Unity.Licensing.EntitlementResolver.dll未找到！");
    Console.ReadKey();
    return;
}

ExecuteCommand($"patch-windows-amd64.exe \"{dllPath}\"");

Console.WriteLine("==========================开始签名==========================");

File.Delete("Unity_lic.ulf");
ExecuteCommand("sign-windows-amd64.exe");

if (!File.Exists("Unity_lic.ulf")) {
    Console.WriteLine("签名失败！");
}

Console.WriteLine("==========================授权拷贝==========================");
File.Copy("Unity_lic.ulf", @"C:\ProgramData\Unity\Unity_lic.ulf", true);

Console.WriteLine("破解完成！");
Console.ReadKey();