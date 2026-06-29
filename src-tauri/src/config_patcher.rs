use std::fs;
use std::path::Path;

/// 创建 Unity 配置文件以禁用授权验证
pub fn create_unity_config() -> Result<String, String> {
    let config_dir = Path::new("C:\\ProgramData\\Unity\\config");
    fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;

    let config_path = config_dir.join("services-config.json");

    let config_content = r#"{
  "enableLicenseValidation": false,
  "enableEntitlementValidation": false,
  "offlineMode": true,
  "disableSignatureValidation": true
}"#;

    fs::write(&config_path, config_content).map_err(|e| e.to_string())?;

    Ok(format!("Created config: {}", config_path.display()))
}

/// 生成无签名的授权文件
pub fn create_unsigned_license() -> Result<String, String> {
    let ulf_path = Path::new("C:\\ProgramData\\Unity\\Unity_lic.ulf");

    // 无签名的授权文件 - 移除 <Signature> 节点
    let ulf_content = r#"<root>
  <License id="Terms">
    <MachineBindings>
      <Binding Key="1" Value="00328-90000-00000-AAOEM"/>
      <Binding Key="2" Value="00000_00_000000_00_1000A7_25_4B456B_65.2"/>
      <Binding Key="4" Value="RFpfVTlSM045NDkz"/>
      <Binding Key="5" Value="b0:41:6f:13:fe:76"/>
    </MachineBindings>
    <MachineID Value="5LduG3msC5HiV8jKZg2HGVycmkY="/>
    <SerialHash Value="b7ba57d463155fb1232555425abce21e9e000a77"/>
    <Features>
      <Feature Value="0"/>
      <Feature Value="1"/>
      <Feature Value="2"/>
      <Feature Value="3"/>
      <Feature Value="4"/>
      <Feature Value="5"/>
      <Feature Value="6"/>
      <Feature Value="9"/>
      <Feature Value="10"/>
      <Feature Value="11"/>
      <Feature Value="12"/>
      <Feature Value="13"/>
      <Feature Value="14"/>
      <Feature Value="15"/>
      <Feature Value="17"/>
      <Feature Value="18"/>
      <Feature Value="19"/>
      <Feature Value="20"/>
      <Feature Value="21"/>
      <Feature Value="23"/>
      <Feature Value="24"/>
      <Feature Value="25"/>
      <Feature Value="26"/>
      <Feature Value="28"/>
      <Feature Value="29"/>
      <Feature Value="30"/>
      <Feature Value="31"/>
      <Feature Value="32"/>
      <Feature Value="33"/>
      <Feature Value="34"/>
      <Feature Value="35"/>
      <Feature Value="36"/>
      <Feature Value="39"/>
      <Feature Value="40"/>
    </Features>
    <DeveloperData Value="AQAAAEY0LUE4UDAtVVdITC1CT0tXLVdHRlEtVkY4TA=="/>
    <SerialMasked Value="F4-A8P0-UWHL-BOKW-WGFQ-XXXX"/>
    <StartDate Value="2026-06-26T03:52:27"/>
    <UpdateDate Value="2096-06-26T03:52:27"/>
    <InitialActivationDate Value="2026-06-25T03:52:27"/>
    <LicenseVersion Value="6.x"/>
    <ClientProvidedVersion Value="2017.2.0"/>
    <AlwaysOnline Value="false"/>
  </License>
</root>"#;

    fs::write(&ulf_path, ulf_content).map_err(|e| e.to_string())?;

    Ok(format!("Created unsigned license: {}", ulf_path.display()))
}

/// 删除配置文件
pub fn remove_unity_config() -> Result<String, String> {
    let config_path = Path::new("C:\\ProgramData\\Unity\\config\\services-config.json");

    if config_path.exists() {
        fs::remove_file(config_path).map_err(|e| e.to_string())?;
        Ok("Removed config file".into())
    } else {
        Ok("Config file not found".into())
    }
}
