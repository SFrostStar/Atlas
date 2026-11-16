using System;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Text;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Windows.Forms;

internal sealed partial class BootstrapWindow
{
    private static readonly Color Accent = Color.FromArgb(255, 139, 62);
    private static readonly Color Night = Color.FromArgb(12, 16, 22);
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
        cancel.SetBounds(width - control - Px(2), 0, control, Px(38));
        status.SetBounds((width - Px(460)) / 2, height / 2 + Px(28), Px(460), Px(31));
        status.Font = new Font("Segoe UI", Px(16), FontStyle.Regular, GraphicsUnit.Pixel);
        cancel.Font = new Font("Segoe UI", Px(17), FontStyle.Regular, GraphicsUnit.Pixel);
        PrepareBackdrop();
    }

    private int Px(int value)
    {
        return Math.Max(1, (int)Math.Round(scale * value));
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
            using (var band = new SolidBrush(Color.FromArgb(142, Night)))
                graphics.FillRectangle(band, 0, 0, bounds.Width, Px(38));
            using (var edge = new Pen(Color.FromArgb(48, Color.White)))
                graphics.DrawLine(edge, 0, Px(38), bounds.Width, Px(38));
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
        if (mark == null) return;
        var graphics = args.Graphics;
        graphics.SmoothingMode = SmoothingMode.AntiAlias;
        graphics.TextRenderingHint = TextRenderingHint.AntiAliasGridFit;
        using (var foreground = new SolidBrush(Color.WhiteSmoke))
        using (var caption = new Font("Segoe UI", Px(13), FontStyle.Regular, GraphicsUnit.Pixel))
        using (var wordmark = new Font("Segoe UI", Px(52), FontStyle.Bold, GraphicsUnit.Pixel))
        using (var format = new StringFormat(StringFormat.GenericTypographic) { Alignment = StringAlignment.Near, LineAlignment = StringAlignment.Center }) {
            graphics.DrawImage(mark, Px(15), Px(10), Px(18), Px(18));
            graphics.DrawString("Atlas Launcher", caption, foreground,
                new RectangleF(Px(42), 0, Px(180), Px(38)), format);

            var textWidth = graphics.MeasureString("Atlas", wordmark, int.MaxValue, format).Width;
            var markSize = Px(62);
            var center = ClientSize.Width / 2f;
            var top = ClientSize.Height / 2f - Px(85);
            var left = center - (markSize + Px(20) + textWidth) / 2f;
            graphics.DrawImage(mark, left, top, markSize, markSize);
            graphics.DrawString("Atlas", wordmark, foreground,
                new RectangleF(left + markSize + Px(20), top, textWidth + Px(10), markSize), format);

            var bar = new RectangleF(center - Px(230), ClientSize.Height / 2f + Px(12), Px(460), Px(5));
            RoundedRail(graphics, bar, Color.FromArgb(72, Color.White));
            if (portion.HasValue) {
                bar.Width *= (float)ProgressAt(motion.ElapsedMilliseconds);
                RoundedRail(graphics, bar, Accent);
            } else {
                var step = SystemInformation.UIEffectsEnabled ? (motion.ElapsedMilliseconds % 1500) / 1500f : .5f;
                var segment = bar.Width * .28f;
                bar.X += (bar.Width - segment) * (.5f - .5f * (float)Math.Cos(step * Math.PI * 2));
                bar.Width = segment;
                RoundedRail(graphics, bar, Accent);
            }
        }
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
