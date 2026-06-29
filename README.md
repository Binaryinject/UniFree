# UniFree 2.0

> Unity Hub & Editor License Patcher - UniHacker True Method

## 🎉 Features

- ✅ **True UniHacker Method** - Replace System.Security.Cryptography.Xml.dll to bypass signature verification
- ✅ **Unity Hub Patcher** - Disable sign-in and auto-update
- ✅ **Unity Editor Scanner** - Auto-detect and patch installed editors
- ✅ **Modern GUI** - Built with Tauri + React + Material-UI
- ✅ **i18n Support** - Chinese & English
- ✅ **Automatic Backup** - All modifications are reversible

## 🚀 Quick Start

### Download

Download the latest release from [Releases](https://github.com/Binaryinject/UniFree/releases)

### Usage

1. **Right-click `unifree.exe` → Run as Administrator**
2. Click **"Hub"** tab
3. Click **"Patch Hub"** button
4. Wait for completion

### Verify

```bash
cd "C:\Program Files\Unity Hub\UnityLicensingClient_V1"
.\Unity.Licensing.Client.exe --showAllEntitlements
```

Expected output:
```
Product Name: Unity Pro
Status: Valid ✅
License Version: 6.x
Update Date: 2096-06-26
```

## 📋 How It Works

### UniHacker True Method

UniFree 2.0 implements the **true UniHacker method**:

1. **Replace DLL**: Replaces `System.Security.Cryptography.Xml.dll` (435 KB → 160 KB cracked version)
2. **Bypass Verification**: The cracked DLL skips XML signature verification
3. **No IL Patching**: No need to modify IL code or PEM certificates

### What Gets Modified

| File | Original | Modified | Backup |
|------|----------|----------|--------|
| System.Security.Cryptography.Xml.dll | 435 KB | 160 KB | ✅ .bak |
| app.asar | 34 MB | 34 MB | ✅ .bak |
| Unity_lic.ulf | - | 2.6 KB | - |

## 🔧 Build from Source

### Prerequisites

- Node.js 18+
- Rust 1.70+
- npm or yarn

### Build Steps

```bash
# Install dependencies
npm install

# Build frontend
npm run build

# Build Tauri app
npm run tauri build
```

## 📚 Documentation

- [Quick Start Guide](QUICK_START.md)
- [UniHacker Method Explained](TRUE_UNIHACKER_METHOD.md)
- [Complete Documentation](FINAL_VERSION_README.md)

## ⚠️ Disclaimer

**For educational purposes only.**

- This tool is for learning and understanding Unity's licensing mechanism
- Use at your own risk
- Consider using Unity Personal (free) or purchasing a legitimate license
- The author is not responsible for any misuse

## 🙏 Credits

- **UniHacker** - Original method inspiration
- **Tauri** - Cross-platform desktop framework
- **React** - UI framework
- **Material-UI** - UI components

## 📊 Project Stats

- **Code Lines**: 1000+
- **Documentation**: 24 files
- **Languages**: Rust, TypeScript, Python
- **Build Size**: 9.3 MB

## 📄 License

MIT License - See [LICENSE](LICENSE) for details

---

**UniFree 2.0** - Unity License Freedom Tool  
Built with ❤️ using Tauri + React + Rust

🎊 **Project Complete!**
