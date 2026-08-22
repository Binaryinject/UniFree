using dnlib.DotNet;
using dnlib.DotNet.Emit;

// usage: diffmethods <original.dll> <patched.dll>
var orig = ModuleDefMD.Load(args[0]);
var patched = ModuleDefMD.Load(args[1]);
try
{
    var oMap = new Dictionary<string, (MethodDef m, List<string> instrs)>();
    foreach (var t in orig.GetTypes())
        foreach (var m in t.Methods)
        {
            var key = $"{t.FullName}::{m.Name} {m.MethodSig}";
            oMap[key] = (m, m.HasBody ? m.Body.Instructions.Select(i => $"{i.OpCode} {i.Operand}").ToList() : null);
        }
    var pMap = new Dictionary<string, (MethodDef m, List<string> instrs)>();
    foreach (var t in patched.GetTypes())
        foreach (var m in t.Methods)
        {
            var key = $"{t.FullName}::{m.Name} {m.MethodSig}";
            pMap[key] = (m, m.HasBody ? m.Body.Instructions.Select(i => $"{i.OpCode} {i.Operand}").ToList() : null);
        }
    Console.WriteLine($"orig methods: {oMap.Count}, patched methods: {pMap.Count}");

    foreach (var kv in oMap)
    {
        if (pMap.TryGetValue(kv.Key, out var pm))
        {
            if (kv.Value.instrs == null && pm.instrs == null) continue;
            if (kv.Value.instrs == null || pm.instrs == null) { Console.WriteLine($"DIFF body-presence: {kv.Key}"); continue; }
            if (!kv.Value.instrs.SequenceEqual(pm.instrs))
            {
                // show first few diff lines
                Console.WriteLine($"DIFF: {kv.Key}");
                int shown = 0;
                for (int i = 0; i < Math.Max(kv.Value.instrs.Count, pm.instrs.Count) && shown < 4; i++)
                {
                    var a = i < kv.Value.instrs.Count ? kv.Value.instrs[i] : "<end>";
                    var b = i < pm.instrs.Count ? pm.instrs[i] : "<end>";
                    if (a != b) { Console.WriteLine($"    [{i}] orig: {a}");
                        Console.WriteLine($"    [{i}] pat:  {b}"); shown++; }
                }
                Console.WriteLine($"    origCount={kv.Value.instrs.Count} patCount={pm.instrs.Count}");
            }
        }
        else
        {
            Console.WriteLine($"MISSING in patched: {kv.Key}");
        }
    }
}
finally { orig.Dispose(); patched.Dispose(); }
