using dnlib.DotNet;
using dnlib.DotNet.Emit;
using dnlib.DotNet.Writer;

// usage: patchresolver <original.dll> <out.dll>
var src = args[0];
var dst = args[1];

var mod = ModuleDefMD.Load(src);
var patched = 0;

foreach (var type in mod.GetTypes())
{
    foreach (var method in type.Methods)
    {
        if (!method.HasBody) continue;
        if (method.Name != "ValidateSignature") continue;

        var instrs = method.Body.Instructions;
        for (int i = 0; i < instrs.Count - 3; i++)
        {
            if (instrs[i].OpCode == OpCodes.Brtrue_S &&
                instrs[i + 1].OpCode == OpCodes.Ldstr &&
                instrs[i + 1].Operand is string s && s == "The digital signature is invalid." &&
                instrs[i + 2].OpCode == OpCodes.Newobj &&
                instrs[i + 3].OpCode == OpCodes.Throw)
            {
                instrs[i].OpCode = OpCodes.Pop;
                instrs[i + 1].OpCode = OpCodes.Nop;
                instrs[i + 1].Operand = null;
                instrs[i + 2].OpCode = OpCodes.Nop;
                instrs[i + 2].Operand = null;
                instrs[i + 3].OpCode = OpCodes.Nop;
                instrs[i + 3].Operand = null;
                patched++;
                Console.WriteLine($"patched {type.FullName}.{method.Name} @ instr {i}");
            }
        }
    }
}

Console.WriteLine($"total ValidateSignature patches: {patched}");
if (patched == 0)
{
    Console.WriteLine("ERROR: no patch applied; output not written");
    return;
}

var opts = new ModuleWriterOptions(mod) { Logger = DummyLogger.NoThrowInstance };
mod.Write(dst, opts);
Console.WriteLine($"wrote {dst} ({new FileInfo(dst).Length} bytes)");
mod.Dispose();
