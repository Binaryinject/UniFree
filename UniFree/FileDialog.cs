using System.Runtime.InteropServices;
using static zFramework.IO.WinAPI;

namespace zFramework.IO; 

public static class FileDialog {
    static string Filter(params string[] filters) {
        return string.Join("\0", filters) + "\0";
    }

    /// <summary>
    /// 打开文件选择窗口
    /// </summary>
    /// <param name="title">指定窗口名称</param>
    /// <param name="extensions">指定文件选择类型，使用 | 分隔</param>
    /// <returns>选中的文件路径的列表</returns>
    /// https://learn.microsoft.com/zh-cn/windows/win32/api/commdlg/ns-commdlg-openfilenamea?redirectedfrom=MSDN
    /// https://www.cnblogs.com/zhaotianff/p/15720189.html
    public static List<string> SelectFile(string title, params string[] extensions) {
        var filters = new List<string> {"All Files", "*.*"};
        foreach (var ext in extensions) {
            if (ext.Contains("|")) {
                var name = ext.Split('|')[0];
                var exts = ext.Split('|')[1];
                filters.Add(name);
                filters.Add(exts);
            }
            else {
                Console.WriteLine($"{nameof(FileDialog)}: 文件扩展参数格式是： 描述+ 竖线+后缀名 ，例如：应用程序|exe");
            }
        }

        var filter = Filter(filters.ToArray());

        int size = 1024;
        List<string> list = new List<string>();
        //多选文件是传出一个指针，这里需要提前分配空间
        //如果是单选文件，使用已经分配大小的StringBuilder或string
        IntPtr filePtr = Marshal.AllocHGlobal(size);

        //清空分配的内存区域
        for (int i = 0; i < size; i++) {
            Marshal.WriteByte(filePtr, i, 0);
        }

        OpenFileName ofn = new OpenFileName();
        ofn.lStructSize = Marshal.SizeOf(ofn);
        ofn.lpstrFilter = filter;
        ofn.nFilterIndex = 2;
        ofn.filePtr = filePtr;
        ofn.nMaxFile = size;
        ofn.nMaxFileTitle = 256;
        ofn.lpstrInitialDir = Environment.GetFolderPath(Environment.SpecialFolder.Desktop);
        ofn.lpstrFileTitle = title;
        ofn.lpstrDefExt = "*.*";
        ofn.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
        ofn.hwndOwner = UnityHWnd; //这一步将文件选择窗口置顶。

        if (GetOpenFileName(ofn)) {
            var file = Marshal.PtrToStringAuto(ofn.filePtr);
            while (!string.IsNullOrEmpty(file)) {
                list.Add(file);
                //转换为地址
                long filePointer = ofn.filePtr;
                //偏移
                filePointer += file.Length * Marshal.SystemDefaultCharSize + Marshal.SystemDefaultCharSize;
                ofn.filePtr = (IntPtr) filePointer;
                file = Marshal.PtrToStringAuto(ofn.filePtr);
            }
        }

        //第一条字符串为文件夹路径，需要再拼成完整的文件路径
        if (list.Count > 1) {
            for (int i = 1; i < list.Count; i++) {
                list[i] = System.IO.Path.Combine(list[0], list[i]);
            }

            list = list.Skip(1).ToList();
        }

        Marshal.FreeHGlobal(filePtr);
        return list;
    }

    /// <summary>
    /// 保存文件选择窗口
    /// </summary>
    /// <param name="title">指定窗口名称</param>
    /// <param name="extensions">预设文件存储位置及文件名</param>
    /// <returns>文件路径</returns>
    public static string? SaveDialog(string title, string path, string extensionDesc) {
        var extension = Path.GetExtension(path);
        OpenFileName ofn = new OpenFileName();
        ofn.lStructSize = Marshal.SizeOf(ofn);
        ofn.lpstrFilter = Filter("All Files", "*.*",
            string.IsNullOrEmpty(extensionDesc) ? extension : extensionDesc, "*" + extension);
        ofn.nFilterIndex = 2;
        var chars = new char[256];
        using var it = Path.GetFileName(path).GetEnumerator();
        for (int i = 0; i < chars.Length && it.MoveNext(); ++i) {
            chars[i] = it.Current;
        }

        var file = new string(chars);
        var filePtr = Marshal.StringToHGlobalAuto(file);
        ofn.filePtr = filePtr;
        ofn.nMaxFile = file.Length;

        ofn.lpstrFileTitle = new string(new char[64]);
        ofn.nMaxFileTitle = 64;
        ofn.lpstrInitialDir = Path.GetDirectoryName(path);
        ofn.lpstrFileTitle = title;
        ofn.lpstrDefExt = "*.*";
        ofn.Flags = OFN_OVERWRITEPROMPT | OFN_HIDEREADONLY | OFN_NOCHANGEDIR;
        ofn.hwndOwner = UnityHWnd; //这一步将文件选择窗口置顶。

        if (!GetSaveFileName(ofn)) {
            return null;
        }

        var saveto = Marshal.PtrToStringUni(ofn.filePtr);
        Marshal.FreeHGlobal(filePtr);
        return saveto;
    }
}