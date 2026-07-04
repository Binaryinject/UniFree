use std::fs;
use std::path::Path;

/// 空签名XML（DLL已绕过验证，签名不需要真实有效）
const DUMMY_SIGNATURE: &str = r##"<Signature xmlns="http://www.w3.org/2000/09/xmldsig#">
  <SignedInfo>
    <CanonicalizationMethod Algorithm="http://www.w3.org/TR/2001/REC-xml-c14n-20010315#WithComments"/>
    <SignatureMethod Algorithm="http://www.w3.org/2000/09/xmldsig#rsa-sha1"/>
    <Reference URI="#Terms">
      <Transforms>
        <Transform Algorithm="http://www.w3.org/2000/09/xmldsig#enveloped-signature"/>
      </Transforms>
      <DigestMethod Algorithm="http://www.w3.org/2000/09/xmldsig#sha1"/>
      <DigestValue>AAAAAAAAAAAAAAAAAAAAAAAAAAA=</DigestValue>
    </Reference>
  </SignedInfo>
  <SignatureValue>AAAAAAAAAAAAAAAAAAAAAAAAAAA=</SignatureValue>
</Signature>"##;

/// 将ALF文件转为ULF（添加空签名节点）
/// 补丁后的DLL绕过了签名验证，不需要真实签名
pub fn sign_alf_to_ulf(alf_path: &Path, ulf_path: &Path) -> Result<String, String> {
    let alf_content = fs::read_to_string(alf_path)
        .map_err(|e| format!("Failed to read ALF: {}", e))?;

    // 在</root>前插入空签名
    let ulf_content = alf_content.replace("</root>", &format!("{}</root>", DUMMY_SIGNATURE));

    fs::write(ulf_path, &ulf_content)
        .map_err(|e| format!("Failed to write ULF: {}", e))?;

    Ok(ulf_path.to_string_lossy().to_string())
}
