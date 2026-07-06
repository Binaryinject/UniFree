use std::fs;
use std::path::Path;

fn main() {
    let versions = ["2019", "2020", "2021", "2022"];
    
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let project_root = Path::new(&manifest_dir).parent().unwrap_or(Path::new("."));
    
    for version in versions {
        let dll_path = project_root.join(format!("src-tauri/resources/win/{}/Unity.Licensing.EntitlementResolver.dll", version));
        let path = dll_path.as_path();
        
        if !path.exists() {
            println!("⚠ {} - file not found, skipping", version);
            continue;
        }
        
        let data = fs::read(path).expect("Failed to read DLL");
        let original_size = data.len();
        
        match patch_assembly(&data) {
            Ok(patched_data) => {
                if patched_data.len() != data.len() || patched_data == data {
                    println!("✗ {} - no changes made", version);
                } else {
                    fs::write(path, &patched_data).expect("Failed to write DLL");
                    println!("✓ {} - patched successfully ({} bytes)", version, original_size);
                }
            }
            Err(e) => {
                println!("✗ {} - error: {}", version, e);
            }
        }
    }
}

/// Patch .NET assembly to bypass signature validation
/// 
/// This function looks for the ValidateSignature method and modifies it to
/// always succeed by patching the IL instructions.
fn patch_assembly(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut result = data.to_vec();
    
    // Find the ValidateSignature method by looking for its name in the metadata
    let method_name = b"ValidateSignature";
    let method_pos = find_pattern(&result, method_name)
        .ok_or("ValidateSignature method not found in metadata")?;
    
    println!("    Found ValidateSignature metadata at offset {}", method_pos);
    
    // In .NET assemblies, the method body is referenced from the MethodDef table
    // The method name is in the #Strings heap, and the IL body is at a separate location
    
    // Strategy: Search for IL code patterns that throw InvalidDataException
    // The typical pattern is:
    //   ldstr <error message token>
    //   newobj InvalidDataException::.ctor(string)
    //   throw
    
    // We'll search for throw (0x2A) instructions and check if they're preceded by
    // exception construction patterns
    
    let mut patches_applied = 0;
    
    // Search for all throw instructions
    for i in 0..result.len() {
        if result[i] == 0x2A { // throw opcode
            // Check if this looks like it's after an exception construction
            // Typical pattern before throw:
            //   73 XX XX XX XX (newobj with 4-byte token)
            // or
            //   72 XX XX XX XX (ldstr)
            //   73 XX XX XX XX (newobj)
            
            let has_newobj_before = i >= 5 && result[i-5] == 0x73;
            let has_ldstr_newobj = i >= 10 && result[i-10] == 0x72 && result[i-5] == 0x73;
            
            if has_newobj_before || has_ldstr_newobj {
                // This looks like an exception throw - NOP it
                result[i] = 0x00; // nop
                patches_applied += 1;
                
                if patches_applied <= 5 {
                    println!("    NOP'd throw at offset {}", i);
                }
            }
        }
    }
    
    // Also look for brtrue.s (0x2D) or brfalse.s (0x2C) after callvirt (0x6F)
    // which would be conditional branches on CheckSignature result
    for i in 0..result.len().saturating_sub(7) {
        if result[i] == 0x6F { // callvirt
            if result[i+5] == 0x2D || result[i+5] == 0x2C { // brtrue.s or brfalse.s
                // Check if this might be related to signature checking
                // by looking for nearby string references
                let nearby = &result[i.saturating_sub(20)..(i+20).min(result.len())];
                if nearby.windows(14).any(|w| w == b"CheckSignature") || 
                   nearby.windows(13).any(|w| w == b"VerifySignature") {
                    // NOP the conditional branch
                    result[i+5] = 0x00;
                    result[i+6] = 0x00;
                    patches_applied += 1;
                    println!("    NOP'd conditional branch at offset {}", i+5);
                }
            }
        }
    }
    
    if patches_applied > 0 {
        println!("    Applied {} patches", patches_applied);
        Ok(result)
    } else {
        // If no patches were applied, try a more aggressive approach
        // Search for the method body directly
        
        // Alternative: Find InvalidDataException usage
        let invalid_exc = b"InvalidDataException";
        if let Some(exc_pos) = find_pattern(&result, invalid_exc) {
            println!("    Found InvalidDataException at offset {}", exc_pos);
            
            // Search for throw instructions near this reference
            let search_start = exc_pos.saturating_sub(0x10000);
            let search_end = (exc_pos + 0x10000).min(result.len());
            
            for i in search_start..search_end {
                if result[i] == 0x2A && i >= 5 && result[i-5] == 0x73 {
                    result[i] = 0x00;
                    patches_applied += 1;
                    println!("    NOP'd throw at offset {}", i);
                }
            }
            
            if patches_applied > 0 {
                println!("    Applied {} patches (aggressive)", patches_applied);
                return Ok(result);
            }
        }
        
        Err("Could not find patchable patterns".to_string())
    }
}

fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    if pattern.is_empty() || data.len() < pattern.len() {
        return None;
    }
    for i in 0..=data.len() - pattern.len() {
        if data[i..i + pattern.len()] == *pattern {
            return Some(i);
        }
    }
    None
}
