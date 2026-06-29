#!/usr/bin/env python3
"""
生成已签名的 Unity 授权文件
使用与 app.asar 中替换的 PEM 证书匹配的私钥
"""

from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa, padding
from cryptography.hazmat.backends import default_backend
import base64

# 授权文件内容（不包含签名）
LICENSE_XML = """<root>
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
</root>"""

def generate_keypair():
    """生成 RSA 2048 密钥对"""
    private_key = rsa.generate_private_key(
        public_exponent=65537,
        key_size=2048,
        backend=default_backend()
    )
    return private_key

def save_keys(private_key, private_path, public_path):
    """保存密钥到文件"""
    # 保存私钥
    pem_private = private_key.private_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PrivateFormat.PKCS8,
        encryption_algorithm=serialization.NoEncryption()
    )
    with open(private_path, 'wb') as f:
        f.write(pem_private)

    # 保存公钥
    public_key = private_key.public_key()
    pem_public = public_key.public_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PublicFormat.SubjectPublicKeyInfo
    )
    with open(public_path, 'wb') as f:
        f.write(pem_public)

    print(f"✓ Keys saved to {private_path} and {public_path}")
    return public_key

def sign_license(license_xml, private_key):
    """签名授权文件"""
    # 提取 License 节点
    license_start = license_xml.find('<License')
    license_end = license_xml.find('</License>') + len('</License>')
    license_content = license_xml[license_start:license_end]

    # 计算 SHA1 digest
    from hashlib import sha1
    digest = sha1(license_content.encode()).digest()
    digest_b64 = base64.b64encode(digest).decode()

    # 构建 SignedInfo
    signed_info = f'''<SignedInfo><CanonicalizationMethod Algorithm="http://www.w3.org/TR/2001/REC-xml-c14n-20010315"/><SignatureMethod Algorithm="http://www.w3.org/2000/09/xmldsig#rsa-sha1"/><Reference URI="#Terms"><Transforms><Transform Algorithm="http://www.w3.org/2000/09/xmldsig#enveloped-signature"/></Transforms><DigestMethod Algorithm="http://www.w3.org/2000/09/xmldsig#sha1"/><DigestValue>{digest_b64}</DigestValue></Reference></SignedInfo>'''

    # 签名 SignedInfo
    signature = private_key.sign(
        signed_info.encode(),
        padding.PKCS1v15(),
        hashes.SHA1()
    )
    signature_b64 = base64.b64encode(signature).decode()

    # 构建完整的 XML
    signed_xml = f'''<root>
  {license_content}
  <Signature xmlns="http://www.w3.org/2000/09/xmldsig#">
    <SignedInfo>
      <CanonicalizationMethod Algorithm="http://www.w3.org/TR/2001/REC-xml-c14n-20010315"/>
      <SignatureMethod Algorithm="http://www.w3.org/2000/09/xmldsig#rsa-sha1"/>
      <Reference URI="#Terms">
        <Transforms>
          <Transform Algorithm="http://www.w3.org/2000/09/xmldsig#enveloped-signature"/>
        </Transforms>
        <DigestMethod Algorithm="http://www.w3.org/2000/09/xmldsig#sha1"/>
        <DigestValue>{digest_b64}</DigestValue>
      </Reference>
    </SignedInfo>
    <SignatureValue>{signature_b64}</SignatureValue>
  </Signature>
</root>'''

    return signed_xml

if __name__ == '__main__':
    import sys

    print("UniFree - XML Signature Generator")
    print("=" * 50)

    # 生成密钥对
    print("Generating RSA 2048 keypair...")
    private_key = generate_keypair()
    public_key = save_keys(
        private_key,
        'private_key.pem',
        'public_key.pem'
    )

    # 签名授权文件
    print("\nSigning license file...")
    signed_license = sign_license(LICENSE_XML, private_key)

    # 保存签名后的授权文件
    with open('Unity_lic_signed.ulf', 'w') as f:
        f.write(signed_license)

    print("✓ Signed license saved to Unity_lic_signed.ulf")
    print("\n" + "=" * 50)
    print("Next steps:")
    print("1. Replace PEM certificate in app.asar with public_key.pem")
    print("2. Copy Unity_lic_signed.ulf to C:\\ProgramData\\Unity\\Unity_lic.ulf")
    print("3. Test with Unity Hub")
