using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;
using System.Web.Script.Serialization;

internal static class UpdateCore
{
    private const string Repository = "SFrostStar/Atlas";
    private const string Api = "https://api.github.com/repos/" + Repository + "/releases/";
    private const string Marker = "atlas-bootstrap-version.txt";
    private const string Exe = "Atlas Launcher.exe";
    private const int JsonLimit = 4 * 1024 * 1024;
    private const int FileLimit = 512 * 1024 * 1024;
    private const int ArchiveLimit = 1024 * 1024 * 1024;
    private const int EntryLimit = 12000;
    private const long TotalLimit = 2L * 1024 * 1024 * 1024;

    private static readonly Regex Sha256 = new Regex(@"\A[0-9a-f]{64}\z", RegexOptions.CultureInvariant);
    private static readonly Regex SafeComponent = new Regex(@"\A[A-Za-z0-9_. $+-]+\z", RegexOptions.CultureInvariant);
    private static readonly Regex Device = new Regex(@"^(?:CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])(?:\.|$)",
        RegexOptions.IgnoreCase | RegexOptions.CultureInvariant);
    private static readonly Regex Locales = new Regex(@"^locales/[A-Za-z0-9_-]+\.pak$", RegexOptions.CultureInvariant);
    private static readonly Regex GameResources = new Regex(
        @"^(?:LICENSE[^/]*\.txt|game-root/exporters/(?:manifest\.json|(?:client|server)/dev/atlaslab/parity/[A-Za-z0-9_$]+\.class)|game-root/mods/atlas/src/test/java/dev/atlaslab/parity/[A-Za-z0-9_$]+\.java)$",
        RegexOptions.CultureInvariant);
    private static readonly HashSet<string> Hosts = new HashSet<string>(StringComparer.Ordinal)
    {
        "api.github.com", "github.com", "release-assets.githubusercontent.com", "objects.githubusercontent.com"
    };
    private static readonly HashSet<string> AppFiles = new HashSet<string>(StringComparer.Ordinal)
    {
        Exe, "chrome_100_percent.pak", "chrome_200_percent.pak", "d3dcompiler_47.dll",
        "dxcompiler.dll", "dxil.dll", "electron_wer.dll", "ffmpeg.dll", "icudtl.dat",
        "LICENSE", "LICENSES.chromium.html", "resources.pak", "snapshot_blob.bin",
        "v8_context_snapshot.bin", "version", "vk_swiftshader_icd.json", "vk_swiftshader.dll",
        "vulkan-1.dll", "resources/app.asar", "atlas-portable.json", "START-HERE.txt"
    };
    private static readonly HashSet<string> GameFiles = new HashSet<string>(StringComparer.Ordinal)
    {
        "atlas-client.exe", "atlas-ctl.exe", "atlas-prepare.exe", "atlas-server.exe", "libc++.dll",
        "libunwind.dll", "libwinpthread-1.dll", "atlas-runtime.json", "game-root/research-lock.json",
        "game-root/ATLAS-UPSTREAM.json", "game-root/crates/atlas-prepare/src/exporters.rs"
    };
    private static readonly string[] RequiredFiles =
    {
        Exe, "resources/app.asar", "atlas-portable.json", "bundle/game/atlas-client.exe",
        "bundle/game/atlas-ctl.exe", "bundle/game/atlas-prepare.exe", "bundle/game/libc++.dll",
        "bundle/game/libunwind.dll", "bundle/game/libwinpthread-1.dll", "bundle/game/atlas-runtime.json",
        "bundle/game/game-root/research-lock.json", "bundle/game/game-root/exporters/manifest.json"
    };

    internal sealed class NoBootstrapReleaseException : InvalidOperationException
    {
        internal NoBootstrapReleaseException() : base("GitHub has not published an Atlas launcher update package yet.") { }
    }

    internal sealed class Update
    {
        internal readonly Version Version;
        internal readonly Uri Archive;
        internal readonly long Size;
        internal readonly string Hash;
        internal readonly Dictionary<string, PackageFile> Files;

        internal Update(Version version, Uri archive, long size, string hash, Dictionary<string, PackageFile> files)
        {
            Version = version;
            Archive = archive;
            Size = size;
            Hash = hash;
            Files = files;
        }
    }

    internal sealed class PackageFile
    {
        public string path { get; set; }
        public long size { get; set; }
        public string sha256 { get; set; }
    }

    private sealed class Manifest
    {
        public int schema { get; set; }
        public string repository { get; set; }
        public string platform { get; set; }
        public string version { get; set; }
        public string tag { get; set; }
        public ArchiveInfo archive { get; set; }
        public PackageFile[] files { get; set; }
    }

    private sealed class ArchiveInfo
    {
        public string name { get; set; }
        public long size { get; set; }
        public string sha256 { get; set; }
    }

    private sealed class Release
    {
        public string tag_name { get; set; }
        public bool draft { get; set; }
        public bool prerelease { get; set; }
        public Asset[] assets { get; set; }
    }

    private sealed class Asset
    {
        public string name { get; set; }
        public string browser_download_url { get; set; }
        public string digest { get; set; }
        public long size { get; set; }
        public string state { get; set; }
    }

    internal static Version ReadInstalledVersion(string root)
    {
        var marker = Guard(Path.Combine(root, Marker));
        if (!File.Exists(marker)) return null;
        using (var stream = new FileStream(marker, FileMode.Open, FileAccess.Read, FileShare.Read | FileShare.Delete))
        {
            if (stream.Length > 64) return null;
            using (var reader = new StreamReader(stream))
            {
                var version = ParseVersion(reader.ReadToEnd().Trim());
                return version != null && File.Exists(LauncherPath(root, version)) ? version : null;
            }
        }
    }

    internal static string LauncherPath(string root, Version version)
    {
        if (version == null || version.Build < 0 || version.Revision >= 0)
            throw new ArgumentException("Invalid Atlas launcher version.", "version");
        return Guard(Path.Combine(root, "versions", version.ToString(), Exe));
    }

    internal static string GuardPath(string root, string path)
    {
        var canonicalRoot = Path.GetFullPath(root);
        var full = Path.GetFullPath(path);
        var prefix = canonicalRoot.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)
            + Path.DirectorySeparatorChar;
        if (!string.Equals(full, canonicalRoot, StringComparison.OrdinalIgnoreCase)
            && !full.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
            throw new IOException("Atlas startup path escapes its installation folder.");
        return Guard(full);
    }

    internal static Task<Update> FindUpdateAsync(Version installed, CancellationToken cancellation, Action<Version> updateFound = null)
    {
        return Task.Run(() => FindUpdate(installed, GetJsonBytes, cancellation, updateFound), cancellation);
    }

    internal static Update FindUpdate(Version installed, Func<Uri, CancellationToken, byte[]> fetch, CancellationToken cancellation, Action<Version> updateFound = null)
    {
        var latest = ReadJson<Release>(fetch(new Uri(Api + "latest"), cancellation));
        var version = latest.tag_name != null && latest.tag_name.StartsWith("atlas-", StringComparison.Ordinal)
            ? ParseVersion(latest.tag_name.Substring("atlas-".Length)) : null;
        if (latest.draft || latest.prerelease || version == null)
            throw new InvalidDataException("GitHub did not return a valid Atlas release.");
        if (installed != null && version <= installed) return null;
        if (!Assets(latest).Any(asset => asset.name == "Atlas-Launcher.exe" && asset.state == "uploaded"))
            throw new NoBootstrapReleaseException();

        cancellation.ThrowIfCancellationRequested();
        if (updateFound != null) updateFound(version);
        cancellation.ThrowIfCancellationRequested();
        var tag = "atlas-payload-" + version;
        var payload = ReadJson<Release>(fetch(new Uri(Api + "tags/" + tag), cancellation));
        if (payload.draft || !payload.prerelease || payload.tag_name != tag)
            throw new InvalidDataException("GitHub returned the wrong Atlas update package.");

        var manifest = GetAsset(payload, "atlas-update.json");
        var archive = GetAsset(payload, ArchiveName(version));
        if (manifest.size <= 0 || manifest.size > JsonLimit)
            throw new InvalidDataException("Invalid Atlas update manifest size.");
        var manifestBytes = fetch(AssetUri(payload.tag_name, manifest), cancellation);
        if (manifestBytes.LongLength != manifest.size || (manifest.digest != null &&
            !DigestMatches(manifest.digest, HashBytes(manifestBytes))))
            throw new InvalidDataException("Atlas update manifest checksum mismatch.");
        var hash = archive.digest != null && archive.digest.StartsWith("sha256:", StringComparison.Ordinal)
            ? archive.digest.Substring("sha256:".Length) : null;
        return ParseManifest(manifestBytes, version, tag, AssetUri(payload.tag_name, archive), archive.size, hash);
    }

    internal static Update ParseManifest(byte[] json, Version version, string tag, Uri archive, long size, string hash)
    {
        var manifest = ReadJson<Manifest>(json);
        if (manifest.schema != 1 || manifest.repository != Repository || manifest.platform != "win32-x64"
            || manifest.version != version.ToString() || manifest.tag != tag || manifest.archive == null)
            throw new InvalidDataException("The update manifest does not match this Atlas release.");
        if (manifest.archive.name != ArchiveName(version) || manifest.archive.size != size ||
            size < 22 || size > ArchiveLimit || hash == null || !Sha256.IsMatch(hash) || manifest.archive.sha256 != hash)
            throw new InvalidDataException("Invalid Atlas update archive or GitHub checksum.");
        if (manifest.files == null || manifest.files.Length > EntryLimit)
            throw new InvalidDataException("Invalid Atlas update inventory.");

        long total = 0;
        var files = new Dictionary<string, PackageFile>(StringComparer.Ordinal);
        var names = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var file in manifest.files)
        {
            if (file == null || !AllowedFile(file.path) || file.size < 0 || file.size > FileLimit
                || file.sha256 == null || !Sha256.IsMatch(file.sha256) || !names.Add(file.path))
                throw new InvalidDataException("Invalid Atlas update inventory.");
            files.Add(file.path, file);
            total += file.size;
        }
        if (total > TotalLimit || RequiredFiles.Any(file => !files.ContainsKey(file)))
            throw new InvalidDataException("Incomplete or excessive Atlas update payload.");
        return new Update(version, archive, size, hash, files);
    }

    internal static Task<Version> InstallAsync(string root, Update update, Action<long, long> progress, CancellationToken cancellation)
    {
        return Task.Run(() =>
        {
            var versions = Guard(Path.Combine(root, "versions"));
            Directory.CreateDirectory(versions);
            var staging = Guard(Path.Combine(versions, ".staging-" + Guid.NewGuid().ToString("N")));
            Directory.CreateDirectory(staging);
            try
            {
                var archive = Path.Combine(staging, "package.zip");
                Request(update.Archive, cancellation, input =>
                {
                    using (var output = NewFile(archive))
                        Verify(Copy(input, output, update.Size, cancellation, progress), update.Size, update.Hash);
                    return true;
                });
                return InstallArchive(root, update, archive, cancellation);
            }
            finally { CleanStaging(versions, staging); }
        }, cancellation);
    }

    internal static Version InstallArchive(string root, Update update, string archive, CancellationToken cancellation)
    {
        var versions = Guard(Path.Combine(root, "versions"));
        Directory.CreateDirectory(versions);
        // Avoid a race between version activation and a second copy of the updater.
        using (new FileStream(Guard(Path.Combine(versions, ".install.lock")), FileMode.OpenOrCreate, FileAccess.Write, FileShare.None))
        {
            var active = ReadInstalledVersion(root);
            if (active != null && active >= update.Version) return active;
            var staging = Guard(Path.Combine(versions, ".staging-" + Guid.NewGuid().ToString("N")));
            var final = Path.GetDirectoryName(LauncherPath(root, update.Version));
            Directory.CreateDirectory(staging);
            try
            {
                using (var input = File.OpenRead(archive))
                {
                    Verify(Copy(input, Stream.Null, update.Size, cancellation, null), update.Size, update.Hash);
                    input.Position = 0;
                    Extract(input, staging, update, cancellation);
                }

                cancellation.ThrowIfCancellationRequested();
                if (Directory.Exists(final))
                {
                    // A previously completed download can have been interrupted just before activation.
                    VerifyExisting(final, update, cancellation);
                }
                else
                {
                    Directory.Move(staging, final);
                }
                Activate(root, update.Version);
                return update.Version;
            }
            finally { CleanStaging(versions, staging); }
        }
    }

    private static void Extract(Stream input, string staging, Update update, CancellationToken cancellation)
    {
        using (var zip = new ZipArchive(input, ZipArchiveMode.Read, true))
        {
            if (zip.Entries.Count != update.Files.Count)
                throw new InvalidDataException("The ZIP does not match the Atlas update inventory.");
            var seen = new HashSet<string>(StringComparer.Ordinal);
            foreach (var entry in zip.Entries)
            {
                cancellation.ThrowIfCancellationRequested();
                PackageFile file;
                if (!update.Files.TryGetValue(entry.FullName, out file) || !seen.Add(entry.FullName)
                    || entry.Length != file.size || (entry.ExternalAttributes & 0x400) != 0
                    || ((entry.ExternalAttributes >> 16) & 0xf000) == 0xa000)
                    throw new InvalidDataException("Unsafe or unexpected file in the Atlas ZIP.");
                var target = Guard(Path.Combine(staging, file.path.Replace('/', Path.DirectorySeparatorChar)));
                Directory.CreateDirectory(WindowsPath(Path.GetDirectoryName(target)));
                using (var source = entry.Open())
                using (var output = NewFile(target))
                    Verify(Copy(source, output, file.size, cancellation, null), file.size, file.sha256);
            }
        }
    }

    private static void VerifyExisting(string folder, Update update, CancellationToken cancellation)
    {
        var count = 0;
        var dirs = new Stack<string>();
        dirs.Push(folder);
        while (dirs.Count > 0)
        {
            var directory = dirs.Pop();
            foreach (var entry in Directory.EnumerateFileSystemEntries(WindowsPath(directory)))
            {
                cancellation.ThrowIfCancellationRequested();
                var safe = Guard(Path.Combine(directory, Path.GetFileName(entry)));
                if ((File.GetAttributes(WindowsPath(safe)) & FileAttributes.Directory) != 0)
                {
                    dirs.Push(safe);
                    continue;
                }
                PackageFile file;
                if (!update.Files.TryGetValue(safe.Substring(folder.Length + 1).Replace('\\', '/'), out file))
                    throw new InvalidDataException("Unexpected file in an existing Atlas version folder.");
                using (var source = File.OpenRead(WindowsPath(safe)))
                    Verify(Copy(source, Stream.Null, file.size, cancellation, null), file.size, file.sha256);
                count++;
            }
        }
        if (count != update.Files.Count)
            throw new InvalidDataException("An existing Atlas version folder is incomplete.");
    }

    private static void Activate(string root, Version version)
    {
        var marker = Guard(Path.Combine(root, Marker));
        var temp = Guard(marker + "." + Guid.NewGuid().ToString("N"));
        try
        {
            using (var stream = NewFile(temp))
            {
                var bytes = Encoding.ASCII.GetBytes(version + Environment.NewLine);
                stream.Write(bytes, 0, bytes.Length);
                stream.Flush(true);
            }
            if (File.Exists(marker)) File.Replace(temp, marker, null);
            else File.Move(temp, marker);
        }
        finally { File.Delete(temp); }
    }

    private static void CleanStaging(string parent, string staging)
    {
        if (!string.Equals(Path.GetDirectoryName(staging), parent, StringComparison.OrdinalIgnoreCase)
            || !Path.GetFileName(staging).StartsWith(".staging-", StringComparison.Ordinal))
            throw new InvalidOperationException("Unexpected Atlas staging path.");
        try
        {
            if (Directory.Exists(staging)) Directory.Delete(WindowsPath(Guard(staging), true), true);
        }
        catch (IOException) { }
        catch (UnauthorizedAccessException) { }
    }

    private static string Guard(string path)
    {
        var full = Path.GetFullPath(path);
        for (var current = full; current != null; current = Path.GetDirectoryName(current))
        {
            try
            {
                if ((File.GetAttributes(WindowsPath(current)) & FileAttributes.ReparsePoint) != 0)
                    throw new IOException("Atlas startup paths cannot contain linked folders.");
            }
            catch (FileNotFoundException) { }
            catch (DirectoryNotFoundException) { }
        }
        return full;
    }

    private static string WindowsPath(string path, bool descendants = false)
    {
        // Extended paths also work when Windows' global long-path option is off.
        if ((!descendants && path.Length < 248) || path.StartsWith(@"\\?\", StringComparison.Ordinal))
            return path;
        return path.StartsWith(@"\\", StringComparison.Ordinal)
            ? @"\\?\UNC\" + path.Substring(2)
            : @"\\?\" + path;
    }

    private static bool AllowedFile(string path)
    {
        const string bundle = "bundle/game/";
        if (string.IsNullOrEmpty(path) || path.Length > 240
            || path.Split('/').Any(part => !SafeComponent.IsMatch(part) || part.EndsWith(".", StringComparison.Ordinal)
                || part.EndsWith(" ", StringComparison.Ordinal) || Device.IsMatch(part)))
            return false;
        if (AppFiles.Contains(path) || Locales.IsMatch(path)) return true;
        if (!path.StartsWith(bundle, StringComparison.Ordinal)) return false;
        var game = path.Substring(bundle.Length);
        return GameFiles.Contains(game) || GameResources.IsMatch(game);
    }

    private static Version ParseVersion(string text)
    {
        Version version;
        return Version.TryParse(text, out version) && version.Build >= 0
            && version.Revision < 0 && version.ToString() == text ? version : null;
    }

    private static string ArchiveName(Version version)
    {
        return "Atlas-" + version + "-Update.zip";
    }

    private static IEnumerable<Asset> Assets(Release release)
    {
        if (release.assets == null) throw new InvalidDataException("GitHub did not return release assets.");
        return release.assets.Where(asset => asset != null);
    }

    private static Asset GetAsset(Release release, string name)
    {
        var assets = Assets(release).Where(asset => asset.name == name && asset.state == "uploaded").ToArray();
        if (assets.Length != 1) throw new InvalidDataException("Missing or duplicate GitHub release asset: " + name);
        return assets[0];
    }

    private static Uri AssetUri(string tag, Asset asset)
    {
        Uri uri;
        if (!Uri.TryCreate(asset.browser_download_url, UriKind.Absolute, out uri)
            || uri.Host != "github.com" || uri.AbsolutePath != "/" + Repository + "/releases/download/" + tag + "/" + asset.name
            || uri.Query.Length != 0 || uri.Fragment.Length != 0)
            throw new InvalidDataException("The update asset does not belong to this Atlas release.");
        return TrustedUri(uri);
    }

    private static Uri TrustedUri(Uri uri)
    {
        if (!uri.IsAbsoluteUri || uri.Scheme != Uri.UriSchemeHttps || uri.Port != 443 || uri.UserInfo.Length != 0
            || !Hosts.Contains(uri.Host))
            throw new InvalidDataException("Untrusted Atlas update address.");
        return uri;
    }

    private static byte[] GetJsonBytes(Uri uri, CancellationToken cancellation)
    {
        return Request(uri, cancellation, input =>
        {
            using (var buffer = new MemoryStream())
            {
                Copy(input, buffer, JsonLimit, cancellation, null);
                return buffer.ToArray();
            }
        });
    }

    private static T Request<T>(Uri uri, CancellationToken cancellation, Func<Stream, T> read)
    {
        ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
        for (var redirects = 0; redirects < 6; redirects++)
        {
            cancellation.ThrowIfCancellationRequested();
            var request = (HttpWebRequest)WebRequest.Create(TrustedUri(uri));
            request.UserAgent = "Atlas-Launcher";
            request.Accept = "application/vnd.github+json, application/octet-stream";
            request.AllowAutoRedirect = false;
            request.Timeout = 15000;
            request.ReadWriteTimeout = 30000;
            using (cancellation.Register(request.Abort))
            {
                try
                {
                    using (var response = (HttpWebResponse)request.GetResponse())
                    {
                        var status = (int)response.StatusCode;
                        if (status == 301 || status == 302 || status == 303 || status == 307 || status == 308)
                        {
                            var location = response.Headers["Location"];
                            Uri redirected;
                            if (string.IsNullOrWhiteSpace(location) || !Uri.TryCreate(uri, location, out redirected))
                                throw new InvalidDataException("The Atlas update redirect has no address.");
                            uri = TrustedUri(redirected);
                            continue;
                        }
                        if (status != 200) throw new WebException("Atlas update request returned HTTP " + status + ".");
                        using (var stream = response.GetResponseStream()) return read(stream);
                    }
                }
                catch (WebException error)
                {
                    if (error.Response != null) error.Response.Dispose();
                    cancellation.ThrowIfCancellationRequested();
                    throw;
                }
                catch (IOException)
                {
                    cancellation.ThrowIfCancellationRequested();
                    throw;
                }
            }
        }
        throw new WebException("Too many Atlas update redirects.");
    }

    private static T ReadJson<T>(byte[] json) where T : class
    {
        if (json == null || json.Length > JsonLimit)
            throw new InvalidDataException("Invalid Atlas update metadata size.");
        var serializer = new JavaScriptSerializer { MaxJsonLength = JsonLimit, RecursionLimit = 16 };
        var value = serializer.Deserialize<T>(new UTF8Encoding(false, true).GetString(json).TrimStart('\uFEFF'));
        if (value == null) throw new InvalidDataException("Missing Atlas update metadata.");
        return value;
    }

    private static FileStream NewFile(string path)
    {
        return new FileStream(WindowsPath(path), FileMode.CreateNew, FileAccess.Write, FileShare.None);
    }

    private sealed class CopyResult
    {
        internal long Size;
        internal string Hash;
    }

    private static CopyResult Copy(Stream input, Stream output, long limit, CancellationToken cancellation, Action<long, long> progress)
    {
        using (var sha = SHA256.Create())
        {
            var buffer = new byte[64 * 1024];
            long bytes = 0, reported = 0;
            while (true)
            {
                cancellation.ThrowIfCancellationRequested();
                var read = input.Read(buffer, 0, buffer.Length);
                if (read == 0) break;
                bytes += read;
                if (bytes > limit) throw new InvalidDataException("Atlas update exceeds its declared size.");
                sha.TransformBlock(buffer, 0, read, buffer, 0);
                output.Write(buffer, 0, read);
                if (progress != null && bytes - reported >= 512 * 1024)
                {
                    progress(bytes, limit);
                    reported = bytes;
                }
            }
            sha.TransformFinalBlock(buffer, 0, 0);
            if (progress != null && bytes != reported) progress(bytes, limit);
            return new CopyResult { Size = bytes, Hash = Hex(sha.Hash) };
        }
    }

    private static void Verify(CopyResult result, long size, string hash)
    {
        if (result.Size != size || result.Hash != hash)
            throw new InvalidDataException("Atlas update size or checksum mismatch.");
    }

    private static string HashBytes(byte[] bytes)
    {
        using (var sha = SHA256.Create()) return Hex(sha.ComputeHash(bytes));
    }

    private static string Hex(byte[] bytes)
    {
        var text = new StringBuilder(bytes.Length * 2);
        foreach (var value in bytes) text.Append(value.ToString("x2", CultureInfo.InvariantCulture));
        return text.ToString();
    }

    private static bool DigestMatches(string digest, string hash)
    {
        return digest == "sha256:" + hash;
    }
}
