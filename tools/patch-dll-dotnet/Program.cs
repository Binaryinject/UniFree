using dnlib.DotNet;
using dnlib.DotNet.Emit;
using dnlib.DotNet.Writer;

string[] versions = ["2019", "2020", "2021", "2022"];
string basePath = "C:/Users/Administrator/Desktop/win";

foreach (var version in versions)
{
    var dllPath = $"{basePath}/{version}/Unity.Licensing.EntitlementResolver.dll";
    var outputPath = $"C:/GIT/UniFree/src-tauri/resources/win/{version}/Unity.Licensing.EntitlementResolver.dll";
    
    try
    {
        var module = ModuleDefMD.Load(dllPath);
        bool patched = false;
        
        foreach (var type in module.GetTypes())
        {
            foreach (var method in type.Methods)
            {
                if (!method.HasBody) continue;
                
                // Patch ValidateSignature
                if (method.Name == "ValidateSignature")
                {
                    method.Body.Instructions.Clear();
                    method.Body.ExceptionHandlers.Clear();
                    method.Body.Variables.Clear();
                    method.Body.Instructions.Add(OpCodes.Ldc_I4_1.ToInstruction());
                    method.Body.Instructions.Add(OpCodes.Ret.ToInstruction());
                    patched = true;
                    Console.WriteLine($"  {version}: Patched {type.Name}.{method.Name}");
                }
                
                // Patch LegacyUnityLicense..ctor (2019) or UlfLicense..ctor (2020+)
                if (method.IsConstructor && 
                    (type.Name == "LegacyUnityLicense" || type.Name == "UlfLicense") &&
                    method.Parameters.Count >= 2)
                {
                    // Keep only the base call
                    var baseCall = method.Body.Instructions.FirstOrDefault(i => i.OpCode == OpCodes.Call);
                    if (baseCall != null)
                    {
                        method.Body.Instructions.Clear();
                        method.Body.ExceptionHandlers.Clear();
                        method.Body.Variables.Clear();
                        method.Body.Instructions.Add(OpCodes.Ldarg_0.ToInstruction());
                        method.Body.Instructions.Add(baseCall);
                        method.Body.Instructions.Add(OpCodes.Ret.ToInstruction());
                        patched = true;
                        Console.WriteLine($"  {version}: Patched {type.Name}.ctor");
                    }
                }
            }
        }
        
        if (patched)
        {
            // Use the most conservative writer options
            var options = new ModuleWriterOptions(module);
            options.Logger = DummyLogger.NoThrowInstance;
            
            module.Write(outputPath, options);
            Console.WriteLine($"✓ {version} - saved ({new System.IO.FileInfo(outputPath).Length} bytes)");
        }
        else
        {
            Console.WriteLine($"✗ {version} - no methods to patch");
        }
    }
    catch (Exception ex)
    {
        Console.WriteLine($"✗ {version} - {ex.Message}");
    }
}
