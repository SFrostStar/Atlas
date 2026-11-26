using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Text;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Windows.Forms;

internal sealed partial class BootstrapWindow
{
    private const int CaptionHeight = 36;
    private static readonly Color Accent = Color.FromArgb(255, 139, 62);
    private static readonly Color Night = Color.FromArgb(12, 16, 22);
    private static readonly Color Caption = Color.FromArgb(72, 16, 22, 29);
    private readonly PrivateFontCollection typefaces = new PrivateFontCollection();
    private readonly List<GCHandle> fontMemory = new List<GCHandle>(2);
    private Font wordmarkFont;
    private Font statusFont;
    private Font closeFont;
    private Bitmap mark;
    private readonly Stopwatch motion = Stopwatch.StartNew();
    private readonly Timer animation = new Timer { Interval = 16 };
    private Bitmap backdrop;
    private double? portion;
    private double progressFrom;
    private long progressStarted;
    private int progressDuration;
    private float scale = 1f;

    [DllImport("user32.dll")]
    private static extern uint GetDpiForWindow(IntPtr handle);

    private static Bitmap LoadArtwork(string name)
    {
        using (var stream = Assembly.GetExecutingAssembly().GetManifestResourceStream(name))
        using (var image = Image.FromStream(stream))
            return new Bitmap(image);
    }

    private void StyleWindow()
    {
        AutoScaleMode = AutoScaleMode.None;
        FormBorderStyle = FormBorderStyle.None;
        StartPosition = FormStartPosition.Manual;
        KeyPreview = true;
        DoubleBuffered = true;
        BackColor = Night;
        ForeColor = Color.WhiteSmoke;
        animation.Tick += delegate {
            if (portion.HasValue && motion.ElapsedMilliseconds - progressStarted >= progressDuration)
                animation.Stop();
            Invalidate();
        };
    }

    private void PlaceWindow()
    {
        var area = Screen.FromPoint(Cursor.Position).WorkingArea;
        Location = area.Location;
        float dpi = 1f;
        try {
            var current = GetDpiForWindow(Handle);
            if (current > 0) dpi = current / 96f;
        } catch (EntryPointNotFoundException) { }
        scale = Math.Min(dpi, Math.Min(area.Width / 1360f, area.Height / 900f));
        int width = Px(1360), height = Px(900);
        Bounds = new Rectangle(area.Left + (area.Width - width) / 2,
            area.Top + (area.Height - height) / 2, width, height);
        int control = Px(44);
        cancel.SetBounds(width - control, 0, control, Px(CaptionHeight));
        var rail = ProgressRail();
        status.SetBounds((int)rail.X, (int)rail.Y + Px(16), (int)rail.Width, Px(31));
        PrepareTypefaces();
        PrepareBackdrop();
    }

    private int Px(int value)
    {
        return Math.Max(1, (int)Math.Round(scale * value));
    }

    private bool CompactBrand { get { return ClientSize.Width <= Px(1200); } }

    private void PrepareTypefaces()
    {
        if (fontMemory.Count == 0) {
            AddTypeface("atlas.geist.regular");
            AddTypeface("atlas.geist.semibold");
        }
        if (wordmarkFont != null) wordmarkFont.Dispose();
        if (statusFont != null) statusFont.Dispose();
        if (closeFont != null) closeFont.Dispose();
        using (var geist = new FontFamily("Geist", typefaces))
        using (var semibold = new FontFamily("Geist SemiBold", typefaces)) {
            wordmarkFont = new Font(semibold, Px(CompactBrand ? 23 : 28), FontStyle.Regular, GraphicsUnit.Pixel);
            statusFont = new Font(geist, Px(15), FontStyle.Regular, GraphicsUnit.Pixel);
            closeFont = new Font(geist, Px(18), FontStyle.Regular, GraphicsUnit.Pixel);
        }
        status.Font = statusFont;
        cancel.Font = closeFont;
    }

    private void AddTypeface(string name)
    {
        using (var stream = Assembly.GetExecutingAssembly().GetManifestResourceStream(name))
        using (var bytes = new MemoryStream()) {
            stream.CopyTo(bytes);
            var memory = GCHandle.Alloc(bytes.ToArray(), GCHandleType.Pinned);
            fontMemory.Add(memory);
            typefaces.AddMemoryFont(memory.AddrOfPinnedObject(), (int)bytes.Length);
        }
    }

    private void DisposeTypefaces()
    {
        if (wordmarkFont != null) wordmarkFont.Dispose();
        if (statusFont != null) statusFont.Dispose();
        if (closeFont != null) closeFont.Dispose();
        typefaces.Dispose();
        foreach (var memory in fontMemory) memory.Free();
        fontMemory.Clear();
    }

    protected override void WndProc(ref Message message)
    {
        const int WM_NCHITTEST = 0x0084, HTCLIENT = 1, HTCAPTION = 2;
        base.WndProc(ref message);
        if (message.Msg != WM_NCHITTEST || message.Result.ToInt32() != HTCLIENT || cancel == null) return;
        var position = message.LParam.ToInt64();
        var point = PointToClient(new Point(unchecked((short)position), unchecked((short)(position >> 16))));
        if (point.Y >= 0 && point.Y < Px(CaptionHeight) && !cancel.Bounds.Contains(point))
            message.Result = new IntPtr(HTCAPTION);
    }

    private RectangleF ProgressRail()
    {
        var width = Math.Min(Px(460), ClientSize.Width - Px(48));
        return new RectangleF((ClientSize.Width - width) / 2f, ClientSize.Height / 2f + Px(12), width, Px(5));
    }

    private void PrepareBackdrop()
    {
        if (backdrop != null) backdrop.Dispose();
        backdrop = new Bitmap(ClientSize.Width, ClientSize.Height);
        using (var landscape = LoadArtwork("atlas.landscape"))
        using (var graphics = Graphics.FromImage(backdrop)) {
            var bounds = new Rectangle(Point.Empty, ClientSize);
            var cover = Math.Max(bounds.Width / (float)landscape.Width, bounds.Height / (float)landscape.Height);
            var width = landscape.Width * cover;
            var height = landscape.Height * cover;
            graphics.InterpolationMode = InterpolationMode.HighQualityBicubic;
            graphics.DrawImage(landscape, (bounds.Width - width) / 2f,
                (bounds.Height - height) * .52f, width, height);
            using (var darkness = new SolidBrush(Color.FromArgb(113, Night)))
                graphics.FillRectangle(darkness, bounds);
            using (var gradient = new LinearGradientBrush(bounds,
                Color.FromArgb(29, Night), Color.FromArgb(184, Night), 90f))
                graphics.FillRectangle(gradient, bounds);
            using (var band = new SolidBrush(Caption))
                graphics.FillRectangle(band, 0, 0, bounds.Width, Px(CaptionHeight));
        }
    }

    protected override void OnPaintBackground(PaintEventArgs args)
    {
        if (backdrop == null) base.OnPaintBackground(args);
        else args.Graphics.DrawImageUnscaled(backdrop, 0, 0);
    }

    protected override void OnPaint(PaintEventArgs args)
    {
        base.OnPaint(args);
        if (mark == null || wordmarkFont == null) return;
        var graphics = args.Graphics;
        graphics.SmoothingMode = SmoothingMode.AntiAlias;
        graphics.InterpolationMode = InterpolationMode.HighQualityBicubic;
        graphics.TextRenderingHint = TextRenderingHint.AntiAliasGridFit;
        var markSize = Px(CompactBrand ? 25 : 30);
        var left = Px(CompactBrand ? 24 : 40);
        var center = Px(CaptionHeight + (CompactBrand ? 45 : 49));
        graphics.DrawImage(mark, new Rectangle(left, center - markSize / 2, markSize, markSize));
        using (var foreground = new SolidBrush(Color.FromArgb(245, 246, 250)))
        using (var format = StringFormat.GenericTypographic) {
            format.LineAlignment = StringAlignment.Center;
            graphics.DrawString("Atlas", wordmarkFont, foreground,
                new RectangleF(left + markSize + Px(CompactBrand ? 7 : 10), center - Px(22), Px(160), Px(44)), format);
        }

        var bar = ProgressRail();
        RoundedRail(graphics, bar, Color.FromArgb(72, Color.White));
        if (portion.HasValue) {
            bar.Width *= (float)ProgressAt(motion.ElapsedMilliseconds);
        } else {
            var step = SystemInformation.UIEffectsEnabled ? (motion.ElapsedMilliseconds % 1500) / 1500f : .5f;
            var segment = bar.Width * .28f;
            bar.X += (bar.Width - segment) * (.5f - .5f * (float)Math.Cos(step * Math.PI * 2));
            bar.Width = segment;
        }
        RoundedRail(graphics, bar, Accent);
    }

    private static void RoundedRail(Graphics graphics, RectangleF bar, Color color)
    {
        if (bar.Width < 1) return;
        var thickness = Math.Min(bar.Height, bar.Width);
        var inset = thickness / 2;
        using (var pen = new Pen(color, thickness) { StartCap = LineCap.Round, EndCap = LineCap.Round })
            graphics.DrawLine(pen, bar.X + inset, bar.Y + bar.Height / 2,
                bar.Right - inset, bar.Y + bar.Height / 2);
    }

    private double ProgressAt(long elapsed)
    {
        if (!portion.HasValue || progressDuration == 0) return portion.GetValueOrDefault();
        var t = Math.Min(1, (elapsed - progressStarted) / (double)progressDuration);
        return progressFrom + (portion.Value - progressFrom) * Ease(t);
    }

    private static double Ease(double t)
    {
        return t * t * (3 - 2 * t);
    }

    private void SetProgress(double? value, int duration = 180)
    {
        var elapsed = motion.ElapsedMilliseconds;
        bool motionEnabled = SystemInformation.UIEffectsEnabled;
        if (value != portion) {
            progressFrom = ProgressAt(elapsed);
            progressStarted = elapsed;
            progressDuration = motionEnabled ? duration : 0;
            portion = value;
        }
        animation.Enabled = Visible && motionEnabled &&
            (!portion.HasValue || elapsed - progressStarted < progressDuration);
        Invalidate();
    }
}
