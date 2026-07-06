using dnlib.DotNet;
using dnlib.DotNet.Emit;

var path = "C:/Users/Administrator/Desktop/win/2021/Unity.Licensing.EntitlementResolver.dll";
var module = ModuleDefMD.Load(path);

// Find the method that checks for xlts
foreach (var type in module.GetTypes())
{
    foreach (var method in type.Methods)
    {
        if (!method.HasBody) continue;
        
        bool hasXlts = false;
        foreach (var instr in method.Body.Instructions)
        {
            if (instr.OpCode == OpCodes.Ldstr && instr.Operand is string s && s.Contains("xlts"))
            {
                hasXlts = true;
                break;
            }
        }
        
        if (hasXlts)
        {
            Console.WriteLine($"{type.FullName}.{method.Name}:");
            Console.WriteLine("  Instructions:");
            for (int i = 0; i < method.Body.Instructions.Count; i++)
            {
                var instr = method.Body.Instructions[i];
                Console.WriteLine($"    [{i}] {instr.OpCode} {instr.Operand}");
            }
        }
    }
}
