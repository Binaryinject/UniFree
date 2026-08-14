using dnlib.DotNet;
using dnlib.DotNet.Emit;

var path = args.Length > 0 ? args[0] : @"D:\Unity\Editor\2019.4.40f1\Editor\Data\Resources\Licensing\Client\Unity.Licensing.EntitlementResolver.dll";
var module = ModuleDefMD.Load(path);

// Find UlfLicense class and dump its Parse + entitlement reading (ldstr only)
foreach (var type in module.GetTypes())
{
    if (type.Name != "UlfLicense") continue;
    Console.WriteLine($"===== TYPE {type.FullName} =====");
    foreach (var method in type.Methods)
    {
        if (method.Name == "Parse" || method.Name == "Read" || method.Name.Contains("Entitlement"))
        {
            Console.WriteLine($"  METHOD {method.Name}");
            if (!method.HasBody) continue;
            foreach (var instr in method.Body.Instructions)
            {
                if (instr.OpCode == OpCodes.Ldstr)
                    Console.WriteLine($"      ldstr {instr.Operand}");
            }
        }
    }
}

// find entitlement-related types and their XML element names
foreach (var type in module.GetTypes())
{
    if (type.FullName.Contains("Entitlement") && type.FullName.Contains("Xml"))
    {
        Console.WriteLine($"\n===== TYPE {type.FullName} =====");
        foreach (var method in type.Methods)
        {
            if (!method.HasBody) continue;
            foreach (var instr in method.Body.Instructions)
            {
                if (instr.OpCode == OpCodes.Ldstr)
                    Console.WriteLine($"  {method.Name}: ldstr {instr.Operand}");
            }
        }
    }
}
