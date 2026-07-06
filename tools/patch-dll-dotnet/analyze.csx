using dnlib.DotNet;

var module = ModuleDefMD.Load("C:/GIT/UniFree/src-tauri/resources/win/2019/Unity.Licensing.EntitlementResolver.dll");

foreach (var type in module.GetTypes())
{
    if (type.FullName.Contains("Legacy") || type.FullName.Contains("License"))
    {
        Console.WriteLine($"Type: {type.FullName}");
        foreach (var method in type.Methods)
        {
            if (method.HasBody)
                Console.WriteLine($"  {method.Name} (RVA: 0x{method.RVA:X})");
        }
    }
}
