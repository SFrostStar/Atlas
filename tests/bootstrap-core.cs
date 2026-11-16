using System;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;

[assembly: System.Runtime.Versioning.TargetFramework(".NETFramework,Version=v4.6.2")]

internal class SelfTest
{
    static readonly string[] Required =
    {
        "Atlas Launcher.exe", "resources/app.asar", "atlas-portable.json", "bundle/game/atlas-client.exe",
        "bundle/game/atlas-ctl.exe", "bundle/game/atlas-prepare.exe", "bundle/game/libc++.dll",
        "bundle/game/libunwind.dll", "bundle/game/libwinpthread-1.dll", "bundle/game/atlas-runtime.json",
        "bundle/game/game-root/research-lock.json", "bundle/game/game-root/exporters/manifest.json"
    };

    static void Equal(object actual, object expected)
    {
        if (!object.Equals(actual, expected)) throw new Exception("Expected " + expected + ", got " + actual);
    }

    static void Rejects<T>(Action action) where T : Exception
    {
        try { action(); }
        catch (T) { return; }
        throw new Exception("Expected " + typeof(T).Name);
    }

    static string Hash(byte[] data)
    {
        using (var sha = SHA256.Create())
            return BitConverter.ToString(sha.ComputeHash(data)).Replace("-", "").ToLowerInvariant();
    }

    static byte[] Json(object value) { return Encoding.UTF8.GetBytes(new JavaScriptSerializer().Serialize(value)); }

    static UpdateCore.Update Package(string root, Version version, out string archive, bool unexpected = false, bool symlink = false)
    {
        var text = version.ToString();
        archive = Path.Combine(root, "Atlas-" + text + "-Update.zip");
        var data = Encoding.ASCII.GetBytes("payload-" + text);
        var files = Required.Select(path => new { path = path, size = data.Length, sha256 = Hash(data) }).ToArray();
        using (var stream = File.Create(archive))
        using (var zip = new ZipArchive(stream, ZipArchiveMode.Create))
        {
            foreach (var file in files)
            {
                var entry = zip.CreateEntry(file.path);
                if (symlink && file.path == Required[0]) entry.ExternalAttributes = unchecked((int)0xA1FF0000);
                using (var output = entry.Open()) output.Write(data, 0, data.Length);
            }
            if (unexpected) zip.CreateEntry("../escape.txt");
        }
        var sha=Hash(File.ReadAllBytes(archive));
        var tag="atlas-payload-"+text;
        var name="Atlas-"+text+"-Update.zip";
        var size=new FileInfo(archive).Length;
        var url=new Uri("https://github.com/SFrostStar/Atlas/releases/download/"+tag+"/"+name);
        var manifest=Json(new { schema=1, repository="SFrostStar/Atlas", platform="win32-x64",version=text,tag=tag,
                                archive=new {name=name,size=size,sha256=sha},files=files});
        var manifestName="atlas-update.json";
        var manifestUri="https://github.com/SFrostStar/Atlas/releases/download/"+tag+"/"+manifestName;
        var latestBytes=Json(new { tag_name="atlas-"+text,draft=false,prerelease=false,
            assets=new[] {new {name="Atlas-Launcher.exe",state="uploaded"}}});
        var payloadBytes=Json(new { tag_name=tag,draft=false,prerelease=true,assets=new[] {
             new {name=manifestName,size=(long)manifest.Length,digest="sha256:"+Hash(manifest),state="uploaded",browser_download_url=manifestUri},
             new {name=name,size=size,digest="sha256:"+sha,state="uploaded",browser_download_url=url.AbsoluteUri}
        }});
        int calls=0, callbacks=0;
        var update=UpdateCore.FindUpdate(null, (uri,token)=>{
             calls++;
             switch(calls)
             {
               case 1: Equal(uri.AbsolutePath,"/repos/SFrostStar/Atlas/releases/latest"); return latestBytes;
               case 2: Equal(callbacks,1); Equal(uri.AbsolutePath,"/repos/SFrostStar/Atlas/releases/tags/"+tag); return payloadBytes;
               default: Equal(uri.AbsoluteUri,manifestUri); return manifest;
             }
        }, CancellationToken.None, discovered=>{Equal(discovered,version);callbacks++;});
        Equal(calls,3);
        calls=0;
        Equal(UpdateCore.FindUpdate(version,(uri,token)=>{calls++;return latestBytes;},CancellationToken.None),null);
        Equal(calls,1);
        return update;
    }

    [STAThread]
    static void Main(string[] args)
    {
        var work=args[0];
        var root=Path.Combine(work,"inst");
        Directory.CreateDirectory(Path.Combine(root,"data"));
        Directory.CreateDirectory(Path.Combine(root,"runtime"));
        File.WriteAllText(Path.Combine(root,"data","world"),"keep");
        File.WriteAllText(Path.Combine(root,"runtime","files"),"keep");
        string first,second,invalid,linked,corrupt,cancelled;
        var v1=new Version("1.3.0");
        var v2=new Version("1.3.1");
        var v3=new Version("1.3.2");
        var v4=new Version("1.3.3");
        var p1=Package(work,v1,out first);
        var p2=Package(work,v2,out second);
        var p3=Package(work,v3,out invalid,true);
        var p4=Package(work,v4,out linked,false,true);
        var corrupted = Package(work, new Version("1.3.4"), out corrupt);
        var cancelledUpdate = Package(work, new Version("1.3.5"), out cancelled);
        var corruptBytes = File.ReadAllBytes(corrupt);
        corruptBytes[0] ^= 1;
        File.WriteAllBytes(corrupt, corruptBytes);

        var marker = Path.Combine(root, "atlas-bootstrap-version.txt");
        Equal(UpdateCore.ReadInstalledVersion(root),null);
        Equal(UpdateCore.InstallArchive(root,p1,first,CancellationToken.None),v1);
        Equal(UpdateCore.ReadInstalledVersion(root),v1);
        Equal(UpdateCore.InstallArchive(root,p2,second,CancellationToken.None),v2);
        Equal(UpdateCore.ReadInstalledVersion(root),v2);
        Equal(UpdateCore.InstallArchive(root,p1,first,CancellationToken.None),v2);
        Equal(File.ReadAllText(UpdateCore.LauncherPath(root, v1)), "payload-1.3.0");

        var activeMarker = File.ReadAllText(marker);
        Rejects<InvalidDataException>(() => UpdateCore.InstallArchive(root, p3, invalid, CancellationToken.None));
        Rejects<InvalidDataException>(() => UpdateCore.InstallArchive(root, p4, linked, CancellationToken.None));
        Rejects<InvalidDataException>(() => UpdateCore.InstallArchive(root, corrupted, corrupt, CancellationToken.None));
        Rejects<OperationCanceledException>(() => UpdateCore.InstallArchive(root, cancelledUpdate, cancelled, new CancellationToken(true)));
        Rejects<IOException>(() => UpdateCore.GuardPath(root, Path.Combine(work, "outside")));
        Equal(File.ReadAllText(marker), activeMarker);
        Equal(UpdateCore.ReadInstalledVersion(root), v2);

        Equal(Directory.GetDirectories(Path.Combine(root,"versions"),".staging-*").Length,0);
        Equal(File.ReadAllText(Path.Combine(root,"data","world")),"keep");
        Equal(File.ReadAllText(Path.Combine(root,"runtime","files")),"keep");
        Equal(File.ReadAllText(UpdateCore.LauncherPath(root,v2)),"payload-1.3.1");
        Equal(File.Exists(Path.Combine(root,"escape.txt")),false);

        var desktop = Path.Combine(work, "desktop");
        Directory.CreateDirectory(desktop);
        var sourceExe = Assembly.GetExecutingAssembly().Location;
        var loader = Path.Combine(root, DesktopEntry.LoaderName);
        var shortcut = Path.Combine(desktop, "Atlas.lnk");
        Equal(DesktopEntry.Install(root, desktop, sourceExe), shortcut);
        Equal(File.ReadAllBytes(loader).SequenceEqual(File.ReadAllBytes(sourceExe)), true);
        Equal(DesktopEntry.Install(root, desktop, loader), shortcut);
        var otherRoot = Path.Combine(work, "other-install");
        Directory.CreateDirectory(otherRoot);
        Equal(DesktopEntry.Install(otherRoot, desktop, sourceExe), Path.Combine(desktop, "Atlas Launcher.lnk"));
        Equal(DesktopEntry.Install(root, desktop, loader), shortcut);
        Console.WriteLine("Bootstrap core passed release checks, installation, upgrades, rollback, data preservation and desktop shortcuts.");
    }
}
