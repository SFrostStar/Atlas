using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;

internal static class DesktopEntry
{
    internal const string LoaderName = "Atlas-Launcher.exe";
    private const string LauncherName = "Atlas Launcher.exe";

    internal static string FindFlatLauncher(string directory)
    {
        var executable = Path.Combine(directory, LauncherName);
        if (File.Exists(executable) &&
            (File.Exists(Path.Combine(directory, "atlas-portable.json")) ||
             File.Exists(Path.Combine(directory, "bundle", "game", "game-root", "research-lock.json"))))
            return UpdateCore.GuardPath(directory, executable);

        var development = Path.Combine(directory, "dist", "Atlas Launcher-win32-x64", LauncherName);
        if (File.Exists(development) && File.Exists(Path.Combine(directory, "package.json")) &&
            File.Exists(Path.Combine(directory, "data", "launcher.json")))
            return UpdateCore.GuardPath(directory, development);
        return null;
    }

    internal static bool SamePath(string left, string right)
    {
        return string.Equals(left, right, StringComparison.OrdinalIgnoreCase);
    }

    internal static string Install(string root, string desktop, string sourceExe)
    {
        var target = UpdateCore.GuardPath(root, Path.Combine(root, LoaderName));
        if (!SamePath(Path.GetFullPath(sourceExe), target) && NeedsCopy(sourceExe, target)) {
            var temporary = target + "." + Guid.NewGuid().ToString("N") + ".tmp";
            try {
                File.Copy(sourceExe, temporary);
                if (File.Exists(target)) File.Replace(temporary, target, null);
                else File.Move(temporary, target);
            } finally {
                if (File.Exists(temporary)) File.Delete(temporary);
            }
        }

        var flatLauncher = FindFlatLauncher(root);
        var shellType = Type.GetTypeFromProgID("WScript.Shell", true);
        object shell = Activator.CreateInstance(shellType);
        try {
            foreach (var name in new[] { "Atlas.lnk", "Atlas Launcher.lnk" }) {
                var path = Path.Combine(desktop, name);
                object shortcut = ((dynamic)shell).CreateShortcut(path);
                try {
                    dynamic link = shortcut;
                    if (File.Exists(path)) {
                        string previousTarget = link.TargetPath;
                        if (SamePath(previousTarget, target)) return path;
                        if (flatLauncher == null || !SamePath(previousTarget, flatLauncher)) continue;
                    }
                    link.TargetPath = target;
                    link.WorkingDirectory = root;
                    link.IconLocation = target + ",0";
                    link.Save();
                    return path;
                } finally {
                    Marshal.FinalReleaseComObject(shortcut);
                }
            }
            throw new IOException("Atlas desktop shortcut names are used by another installation.");
        } finally {
            Marshal.FinalReleaseComObject(shell);
        }
    }

    private static bool NeedsCopy(string source, string target)
    {
        if (!File.Exists(target)) return true;
        Version oldVersion;
        Version newVersion;
        return !Version.TryParse(FileVersionInfo.GetVersionInfo(target).ProductVersion, out oldVersion)
            || (Version.TryParse(FileVersionInfo.GetVersionInfo(source).ProductVersion, out newVersion) && newVersion > oldVersion);
    }
}
