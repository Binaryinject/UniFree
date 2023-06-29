using System.Runtime.InteropServices;
using System.Text;

namespace zFramework.IO {
    public static class WinAPI {
        static IntPtr ptr;

        public static IntPtr UnityHWnd {
            get {
                if (ptr == IntPtr.Zero) {
                    ptr = GetUnityWindow();
                }

                return ptr;
            }
        }

        #region const fields

        public const int OFN_READONLY = 0x1;
        public const int OFN_OVERWRITEPROMPT = 0x2;
        public const int OFN_HIDEREADONLY = 0x4;
        public const int OFN_NOCHANGEDIR = 0x8;
        public const int OFN_SHOWHELP = 0x10;
        public const int OFN_ENABLEHOOK = 0x20;
        public const int OFN_ENABLETEMPLATE = 0x40;
        public const int OFN_ENABLETEMPLATEHANDLE = 0x80;
        public const int OFN_NOVALIDATE = 0x100;
        public const int OFN_ALLOWMULTISELECT = 0x200;
        public const int OFN_EXTENSIONDIFFERENT = 0x400;
        public const int OFN_PATHMUSTEXIST = 0x800;
        public const int OFN_FILEMUSTEXIST = 0x1000;
        public const int OFN_CREATEPROMPT = 0x2000;
        public const int OFN_SHAREAWARE = 0x4000;
        public const int OFN_NOREADONLYRETURN = 0x8000;
        public const int OFN_NOTESTFILECREATE = 0x10000;
        public const int OFN_NONETWORKBUTTON = 0x20000;
        public const int OFN_NOLONGNAMES = 0x40000;
        public const int OFN_EXPLORER = 0x80000;
        public const int OFN_NODEREFERENCELINKS = 0x100000;
        public const int OFN_LONGNAMES = 0x200000;
        public const int OFN_ENABLEINCLUDENOTIFY = 0x400000;
        public const int OFN_ENABLESIZING = 0x800000;
        public const int OFN_DONTADDTORECENT = 0x2000000;
        public const int OFN_FORCESHOWHIDDEN = 0x10000000;
        public const int OFN_EX_NOPLACESBAR = 0x1;
        public const int OFN_SHAREFALLTHROUGH = 2;
        public const int OFN_SHARENOWARN = 1;

        public const int OFN_SHAREWARN = 0;

        // Name of the Unity window class used to find the window handle.
        public const string UNITY_WND_CLASSNAME = "UnityWndClass";

        #endregion

        #region Win32 Warpper

        [DllImport("comdlg32.dll", SetLastError = true, CharSet = CharSet.Auto)]
        public static extern bool GetOpenFileName([In, Out] OpenFileName ofn);

        [DllImport("comdlg32.dll", SetLastError = true, CharSet = CharSet.Auto)]
        public static extern bool GetSaveFileName([In, Out] OpenFileName ofn); //这个方法名称必须为GetSaveFileName

        [DllImport("user32.dll")]
        public static extern IntPtr GetForegroundWindow();

        // 通过将每个窗口的句柄依次传递给应用程序定义的回调函数，枚举与线程关联的所有非子窗口。
        [DllImport("user32.dll")]
        private static extern bool EnumThreadWindows(uint dwThreadId, EnumWindowsProc lpEnumFunc, IntPtr lParam);

        private delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

        //检索调用线程的线程标识符。
        [DllImport("kernel32.dll")]
        private static extern uint GetCurrentThreadId();

        // 检索指定窗口所属的类的名称。
        [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern int GetClassName(IntPtr hWnd, StringBuilder lpString, int nMaxCount);

        #endregion

        #region Functions

        public static IntPtr GetUnityWindow() {
            var unityHWnd = IntPtr.Zero;
            EnumThreadWindows(GetCurrentThreadId(), (hWnd, _) => {
                var classText = new StringBuilder(UNITY_WND_CLASSNAME.Length + 1);
                GetClassName(hWnd, classText, classText.Capacity);

                if (classText.ToString() == UNITY_WND_CLASSNAME) {
                    unityHWnd = hWnd;
                    return false;
                }

                return true;
            }, IntPtr.Zero);
            return unityHWnd;
        }

        #endregion

        #region Assistant Structures

        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
        public struct OpenFileName {
            public int lStructSize;
            public IntPtr hwndOwner;
            public IntPtr hInstance;
            public string lpstrFilter;
            public string lpstrCustomFilter;
            public int nMaxCustFilter;
            public int nFilterIndex;
            public IntPtr filePtr; //多选文件时不能用string或StringBuilder
            public int nMaxFile;
            public string lpstrFileTitle;
            public int nMaxFileTitle;
            public string? lpstrInitialDir;
            public string lpstrTitle;
            public int Flags;
            public short nFileOffset;
            public short nFileExtension;
            public string lpstrDefExt;
            public IntPtr lCustData;
            public IntPtr lpfnHook;
            public string lpTemplateName;
            public IntPtr pvReserved;
            public int dwReserved;
            public int flagsEx;
        }

        #endregion
    }
}