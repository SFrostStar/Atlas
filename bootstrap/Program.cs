using System;
using System.Diagnostics;
using System.Drawing;
using System.Globalization;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using System.Windows.Forms;

internal static class Program
{
    [STAThread]
    private static void Main()
    {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        Application.Run(new BootstrapWindow());
    }
}

internal sealed partial class BootstrapWindow : Form
{
    private const double ReadyProgress = .92;
    private const int HandoffDuration = 500;
    private readonly bool russian = CultureInfo.CurrentUICulture.TwoLetterISOLanguageName == "ru"
        || CultureInfo.CurrentUICulture.TwoLetterISOLanguageName == "uk";
    private readonly CancellationTokenSource stop = new CancellationTokenSource();
    private readonly Label status;
    private readonly Button cancel;
    private bool finished;

    internal BootstrapWindow()
    {
        Text = "Atlas Launcher";
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        StyleWindow();
        MaximizeBox = false;
        MinimizeBox = false;
        ShowInTaskbar = false;
        Opacity = 0;
        status = new Label {
            AutoEllipsis = true, BackColor = Color.Transparent,
            ForeColor = Color.FromArgb(220, 225, 234), TextAlign = ContentAlignment.MiddleCenter,
            AccessibleRole = AccessibleRole.StaticText, UseCompatibleTextRendering = true
        };
        cancel = new Button {
            Text = "×", AccessibleName = Copy("Отмена", "Cancel"),
            FlatStyle = FlatStyle.Flat, ForeColor = ForeColor, BackColor = Color.Transparent,
            UseCompatibleTextRendering = true
        };
        cancel.FlatAppearance.BorderSize = 0;
        cancel.FlatAppearance.MouseOverBackColor = Color.FromArgb(122, 75, 37, 31);
        cancel.FlatAppearance.MouseDownBackColor = Color.FromArgb(160, 75, 37, 31);
        cancel.Click += delegate { Close(); };
        CancelButton = cancel;
        Controls.Add(status);
        Controls.Add(cancel);
        Shown += Run;
    }

    private string Copy(string ru, string en)
    {
        return russian ? ru : en;
    }

    private void ShowProgress(string text)
    {
        if (stop.IsCancellationRequested) return;
        status.Text = text;
        if (!Visible) {
            if (mark == null) mark = LoadArtwork("atlas.mark");
            PlaceWindow();
            ShowInTaskbar = true;
            Opacity = 1;
            Show();
        }
        SetProgress(null);
    }

    private void ReportBytes(long received, long total)
    {
        if (stop.IsCancellationRequested) return;
        if (received >= total) {
            status.Text = Copy("Установка обновления…", "Installing update…");
            SetProgress(ReadyProgress);
            return;
        }
        status.Text = string.Format(CultureInfo.CurrentCulture,
            Copy("Скачивание обновления · {0:N0} / {1:N0} МБ", "Downloading update · {0:N0} / {1:N0} MB"),
            received / 1048576.0, total / 1048576.0);
        SetProgress(ReadyProgress * received / total);
    }

    private async void Run(object sender, EventArgs args)
    {
        Hide();
#if ATLAS_PREVIEW
        try { await Preview(); }
        catch (OperationCanceledException) { }
        catch (Exception error) { MessageBox.Show(error.Message, Text); }
        finished = true;
        Close();
#else
        var ui = SynchronizationContext.Current;
        while (!stop.IsCancellationRequested) {
            string fallback = null;
            try {
                var besideBootstrap = AppDomain.CurrentDomain.BaseDirectory;
                var flatLauncher = DesktopEntry.FindFlatLauncher(besideBootstrap);
                var root = flatLauncher != null || File.Exists(Path.Combine(besideBootstrap, "atlas-bootstrap-version.txt"))
                    ? Path.GetFullPath(besideBootstrap)
                    : Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Atlas");
                Directory.CreateDirectory(UpdateCore.GuardPath(root, root));
                using (var updateLock = File.Open(UpdateCore.GuardPath(root, Path.Combine(root, "atlas-bootstrap.lock")),
                    FileMode.OpenOrCreate, FileAccess.ReadWrite, FileShare.None)) {
                    var installed = UpdateCore.ReadInstalledVersion(root);
                    fallback = installed == null ? flatLauncher : UpdateCore.LauncherPath(root, installed);
                    Version current = installed;
                    if (current == null && fallback != null)
                        Version.TryParse(FileVersionInfo.GetVersionInfo(fallback).ProductVersion, out current);
                    if (fallback == null) ShowProgress(Copy("Проверка версии…", "Checking for updates…"));

                    UpdateCore.Update update;
                    using (var check = CancellationTokenSource.CreateLinkedTokenSource(stop.Token)) {
                        if (fallback != null) check.CancelAfter(1500);
                        update = await UpdateCore.FindUpdateAsync(current, check.Token, version => {
                            check.CancelAfter(Timeout.Infinite);
                            ui.Post(_ => ShowProgress(Copy("Подготовка обновления…", "Preparing the update…")), null);
                        });
                    }
                    if (update != null) {
                        ShowProgress(Copy("Скачивание обновления…", "Downloading update…"));
                        var downloadProgress = new Progress<Tuple<long, long>>(bytes => ReportBytes(bytes.Item1, bytes.Item2));
                        installed = await UpdateCore.InstallAsync(root, update,
                            (received, total) => ((IProgress<Tuple<long, long>>)downloadProgress).Report(Tuple.Create(received, total)),
                            stop.Token);
                        fallback = UpdateCore.LauncherPath(root, installed);
                    }
                    stop.Token.ThrowIfCancellationRequested();
                    if (fallback == null) throw new InvalidOperationException(Copy(
                        "Готовой версии Atlas для установки пока нет.", "No installable Atlas release is available yet."));
                    if (installed != null && (update != null ||
                        !DesktopEntry.SamePath(Application.ExecutablePath, Path.Combine(root, DesktopEntry.LoaderName)))) {
                        try {
                            DesktopEntry.Install(root, Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory),
                                Application.ExecutablePath);
                        } catch (Exception error) {
                            MessageBox.Show(Copy("Не удалось создать ярлык Atlas на рабочем столе.",
                                "Could not create the Atlas desktop shortcut.") + Environment.NewLine + error.Message,
                                Text, MessageBoxButtons.OK, MessageBoxIcon.Warning);
                        }
                    }
                    if (Visible && installed != null) await Handoff(fallback);
                    else StartLauncher(fallback);
                }
                break;
            } catch (Exception error) {
                if (stop.IsCancellationRequested) break;
                if (fallback != null && !Visible) {
                    try { StartLauncher(fallback); break; }
                    catch (Exception launchError) { error = launchError; }
                }
                Hide();
                animation.Stop();
                var detail = error is UpdateCore.NoBootstrapReleaseException
                    ? Copy("На GitHub пока нет релиза для нового загрузчика.", "GitHub does not have a release for the new loader yet.")
                    : error.Message;
                var message = Copy("Не удалось открыть или обновить Atlas.", "Could not launch or update Atlas.")
                    + Environment.NewLine + Environment.NewLine + detail;
                if (fallback != null) {
                    message += Environment.NewLine + Environment.NewLine
                        + Copy("Открыть установленную версию?", "Open the installed version?");
                    if (MessageBox.Show(message, Text, MessageBoxButtons.YesNo, MessageBoxIcon.Warning) == DialogResult.Yes) {
                        try { StartLauncher(fallback); }
                        catch (Exception launchError) { MessageBox.Show(launchError.Message, Text, MessageBoxButtons.OK, MessageBoxIcon.Error); }
                    }
                    break;
                }
                if (MessageBox.Show(message, Text, MessageBoxButtons.RetryCancel, MessageBoxIcon.Error) != DialogResult.Retry) break;
            }
        }
        finished = true;
        Close();
#endif
    }

    private static void StartLauncher(string executable)
    {
        StartLauncherProcess(executable, null, null, null).Dispose();
    }

    private static Process StartLauncherProcess(string executable, string token, string temporary, Rectangle? bounds)
    {
        var start = new ProcessStartInfo(executable) {
            WorkingDirectory = Path.GetDirectoryName(executable), UseShellExecute = false
        };
        start.EnvironmentVariables["ATLAS_LAUNCHER_NO_SELF_UPDATE"] = "1";
        if (token != null && bounds.HasValue) {
            var area = bounds.Value;
            start.EnvironmentVariables["TEMP"] = temporary;
            start.EnvironmentVariables["ATLAS_BOOTSTRAP_HANDOFF"] = token;
            start.EnvironmentVariables["ATLAS_BOOTSTRAP_BOUNDS"] = string.Format(CultureInfo.InvariantCulture,
                "{0},{1},{2},{3}", area.X, area.Y, area.Width, area.Height);
        }
#if ATLAS_PREVIEW
        var profile = Path.GetFullPath(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "preview-profile"));
        start.Arguments = "--user-data-dir=\"" + profile + "\"";
#endif
        var process = Process.Start(start);
        if (process == null) throw new InvalidOperationException("Could not start Atlas Launcher.");
        return process;
    }

    private async Task Handoff(string executable)
    {
        SetProgress(ReadyProgress);
        status.Text = Copy("Открываем лаунчер…", "Opening the launcher…");
        cancel.Enabled = false;
        var token = Guid.NewGuid().ToString("N");
        var temporary = Path.GetTempPath();
        var ready = Path.Combine(temporary, "atlas-handoff-" + token + ".ready");
        TopMost = true;
        try {
            using (var process = StartLauncherProcess(executable, token, temporary, Bounds)) {
                var elapsed = Stopwatch.StartNew();
                while (!File.Exists(ready) && !process.HasExited && elapsed.ElapsedMilliseconds < 5500 &&
                    !stop.IsCancellationRequested)
                    await Task.Delay(25);
                SetProgress(1, HandoffDuration);
                if (SystemInformation.UIEffectsEnabled) {
                    elapsed.Restart();
                    while (elapsed.ElapsedMilliseconds < HandoffDuration && !stop.IsCancellationRequested) {
                        var t = elapsed.ElapsedMilliseconds / (double)HandoffDuration;
                        Opacity = 1 - Ease(t);
                        await Task.Delay(16);
                    }
                }
                Opacity = 0;
            }
        } finally {
            TopMost = false;
            if (File.Exists(ready)) File.Delete(ready);
        }
    }

#if ATLAS_PREVIEW
    private async Task Preview()
    {
        var root = AppDomain.CurrentDomain.BaseDirectory;
        var installed = UpdateCore.ReadInstalledVersion(root);
        if (installed == null) throw new InvalidOperationException(Copy("Папка тестового лаунчера отсутствует.", "The preview launcher folder is missing."));
        ShowProgress(Copy("Предпросмотр обновления…", "Previewing the update…"));
        SetProgress(ReadyProgress, 1900);
        await Task.Delay(1900, stop.Token);
        await Handoff(UpdateCore.LauncherPath(root, installed));
    }
#endif

    protected override void OnFormClosing(FormClosingEventArgs args)
    {
        if (!finished && args.CloseReason == CloseReason.UserClosing) {
            args.Cancel = true;
            cancel.Enabled = false;
            status.Text = Copy("Отмена…", "Cancelling…");
            stop.Cancel();
        }
        base.OnFormClosing(args);
    }

    protected override void Dispose(bool disposing)
    {
        if (disposing) {
            animation.Dispose();
            if (backdrop != null) backdrop.Dispose();
            if (mark != null) mark.Dispose();
            stop.Cancel();
            if (finished) stop.Dispose();
        }
        base.Dispose(disposing);
        if (disposing) DisposeTypefaces();
    }
}
